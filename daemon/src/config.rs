use std::{ops::Deref, path::PathBuf, sync::Arc};

use api::auth::UserInfo;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io,
    sync::RwLock,
};

use crate::Error;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    user: Option<UserInfo>,
    last_login_url: Option<String>,
    logout_url_base: Option<String>,
}

impl Config {
    pub fn user(&self) -> Option<&UserInfo> {
        self.user.as_ref()
    }

    pub fn set_user(&mut self, user: UserInfo) {
        self.user = Some(user);
    }

    pub fn set_last_url(&mut self, url: String) {
        self.last_login_url = Some(url);
    }

    pub fn last_url(&self) -> Option<&str> {
        self.last_login_url.as_deref()
    }

    pub fn set_logout_url_base(&mut self, url: String) {
        self.logout_url_base = Some(url);
    }

    pub fn logout_url_base(&self) -> Option<&str> {
        self.logout_url_base.as_deref()
    }
}

pub(crate) trait AppConfig: Sized {
    fn new(conf: Config, path: PathBuf) -> Self;
    fn config(&self) -> &Config;
    fn config_mut(&mut self) -> &mut Config;
    fn config_path(&self) -> &PathBuf;
    fn config_path_mut(&mut self) -> &mut PathBuf;

    async fn get_or_create_path() -> io::Result<(PathBuf, bool)> {
        let dir = config_dir().unwrap().join("htu-net");
        if !dir.exists() {
            fs::create_dir_all(&dir).await?;
        }
        let path = dir.join("config.json");
        if !path.exists() {
            File::create(&path).await?;
            return Ok((path, true));
        }
        Ok((path, false))
    }

    async fn load_or_create() -> Result<Self, Error> {
        let (path, created) = Self::get_or_create_path().await.map_err(Error::TokioIo)?;
        if created {
            Ok(Self::new(Config::default(), path))
        } else {
            Ok(Self::new(
                serde_json::from_slice(
                    &fs::read(&path)
                        .await
                        .map(|data| if data.is_empty() { "{}".into() } else { data })
                        .map_err(Error::TokioIo)?,
                )
                .map_err(Error::SerdeJson)?,
                path,
            ))
        }
    }

    async fn save(&self) -> io::Result<()> {
        let (path, _) = Self::get_or_create_path().await?;
        fs::write(path, serde_json::to_vec_pretty(self.config())?).await
    }
}

pub(crate) trait AppState {
    fn running(&self) -> bool;
    fn stop(&mut self);
}

#[derive(Debug, Clone)]
pub struct AppInfo {
    config: Config,
    path: PathBuf,
    running: bool,
}

impl AppConfig for AppInfo {
    fn new(conf: Config, path: PathBuf) -> Self {
        Self {
            config: conf,
            path,
            running: true,
        }
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    fn config_path(&self) -> &PathBuf {
        &self.path
    }

    fn config_path_mut(&mut self) -> &mut PathBuf {
        &mut self.path
    }
}

impl AppState for AppInfo {
    fn running(&self) -> bool {
        self.running
    }

    fn stop(&mut self) {
        self.running = false;
    }
}

impl AppInfo {
    pub fn global(self) -> GlobalAppInfo {
        GlobalAppInfo(Arc::new(RwLock::new(self)))
    }
}

#[derive(Debug, Clone)]
pub struct GlobalAppInfo(Arc<RwLock<AppInfo>>);

impl Deref for GlobalAppInfo {
    type Target = Arc<RwLock<AppInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl GlobalAppInfo {
    #[cfg(feature = "auto-update")]
    pub async fn run_auto_update(
        &self,
    ) -> Result<(notify::RecommendedWatcher, tokio::task::JoinHandle<()>), notify::Error> {
        use notify::{event::AccessMode, Event, Watcher};
        use tokio::sync::mpsc;

        let app_info = self.clone();
        let (tx, mut rx) = mpsc::unbounded_channel::<()>();
        let conf_path = app_info.read().await.config_path().clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
            Ok(r) => {
                if let notify::EventKind::Access(notify::event::AccessKind::Close(
                    AccessMode::Write,
                )) = r.kind
                {
                    if r.paths.iter().any(|p| *p == conf_path) {
                        println!("config file updated");
                        tx.send(()).unwrap();
                    }
                }
            }
            Err(e) => eprintln!("watch err:{:?}", e),
        })?;
        let app_info_inner = app_info.clone();
        let handle = tokio::spawn(async move {
            println!("config file updater started");
            let conf_path = app_info_inner.read().await.config_path().clone();
            while rx.recv().await.is_some() && app_info_inner.running().await {
                if let Ok(data) = fs::read(&conf_path).await {
                    #[cfg(debug_assertions)]
                    println!("config file updated, parsing...");
                    match serde_json::from_slice(&data) {
                        Ok(conf) => {
                            *app_info.write().await.config_mut() = conf;
                            #[cfg(debug_assertions)]
                            println!("config updated successfully");
                            #[cfg(feature = "sys-notify")]
                            crate::daemon::notify("配置文件已更新").await;
                        }
                        Err(e) => {
                            eprintln!("Error parsing config: {}", e);
                        }
                    };
                }
            }
            println!("config file updater stopped");
        });
        watcher.watch(
            self.read().await.config_path().parent().unwrap(),
            notify::RecursiveMode::NonRecursive,
        )?;

        Ok((watcher, handle))
    }

    pub async fn running(&self) -> bool {
        self.0.read().await.running()
    }
}
