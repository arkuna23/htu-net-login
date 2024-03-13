use std::{
    path::PathBuf, sync::Arc
};

use api::auth::UserInfo;
use dirs::config_dir;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io, sync::{Mutex, MutexGuard},
};

use crate::Error;

#[derive(Debug, Clone)]
pub struct ConfigFile<T>
where
    T: Serialize + DeserializeOwned + Default,
{
    config: T,
    path: PathBuf,
}

impl Config {
    pub fn user(&self) -> Option<&UserInfo> {
        self.user.as_ref()
    }

    pub fn set_user(&mut self, user: UserInfo) {
        self.user = Some(user);
    }
}

impl<'a, T: Serialize + DeserializeOwned + Default> ConfigFile<T> {
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

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub async fn load_or_create() -> Result<Self, Error> {
        let (path, created) = Self::get_or_create_path().await.map_err(Error::TokioIo)?;
        if created {
            Ok(Self {
                config: T::default(),
                path,
            })
        } else {
            Ok(Self {
                config: serde_json::from_slice(&fs::read(&path).await.map(|data| if data.is_empty() {
                    "{}".into()
                } else {
                    data
                }).map_err(Error::TokioIo)?)
                    .map_err(Error::SerdeJson)?,
                path,
            })
        }
    }

    pub fn config(&self) -> &T {
        &self.config
    }

    pub async fn save(&self) -> io::Result<()> {
        let (path, _) = Self::get_or_create_path().await?;
        fs::write(path, serde_json::to_vec(&self.config)?).await
    }
}

impl ConfigFile<Config> {
    pub async fn with_lock(self) -> ConfigFile<ConfigWithLock> {
        ConfigFile {
            config: ConfigWithLock(Arc::new(Mutex::new(self.config))),
            path: self.path,
        }
    }   
}

impl ConfigFile<ConfigWithLock> {
    #[cfg(feature = "auto-update")]
    pub async fn with_auto_update<'de>(self) -> Result<ConfigFile<ConfigWithLock>, notify::Error> {
        Ok(ConfigFile {
            config: self.config.run_auto_update(&self.path).await?,
            path: self.path,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    user: Option<UserInfo>,
}

#[derive(Debug, Clone)]
pub struct ConfigWithLock(Arc<Mutex<Config>>);

impl ConfigWithLock {
    pub async fn lock(&self) -> MutexGuard<'_, Config> {
        self.0.lock().await
    }
}

impl Serialize for ConfigWithLock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.blocking_lock().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ConfigWithLock {
    fn deserialize<D>(deserializer: D) -> Result<ConfigWithLock, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(ConfigWithLock(Arc::new(Mutex::new(Config::deserialize(
            deserializer,
        )?))))
    }
}

impl Default for ConfigWithLock {
    fn default() -> Self {
        ConfigWithLock(Arc::new(Mutex::new(Config::default())))
    }
}

#[cfg(feature = "auto-update")]
impl ConfigWithLock {
    pub(crate) async fn run_auto_update(self, path: &PathBuf) -> notify::Result<ConfigWithLock> {
        use notify::{RecommendedWatcher, Watcher};
        let arc = self.clone();
        let conf_path = path.to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        let (async_tx, mut async_rx) = tokio::sync::mpsc::channel::<notify::Event>(4);
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;
        watcher.watch(&conf_path, notify::RecursiveMode::NonRecursive)?;
        std::thread::spawn(move || {
            for res in rx {
                match res {
                    Ok(event) => async_tx.blocking_send(event).unwrap(),
                    Err(e) => {
                        eprintln!("watch error: {:?}", e);
                    }
                }
            }
        });

        let arc_inner = arc.clone();
        tokio::spawn(async move {
            while let Some(event) = async_rx.recv().await {
                if let notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) = event.kind {
                    if let Ok(data) = fs::read(&conf_path).await {
                        match serde_json::from_slice(&data) {
                            Ok(conf) => {
                                let mut config = arc_inner.lock().await;
                                *config = conf;
                                #[cfg(feature = "sys-notify")]
                                crate::daemon::notify("配置文件已更新").await;
                            }
                            Err(e) => {
                                eprintln!("Error parsing config: {}", e);
                            }
                        };
                    }
                }
            }
        });

        Ok(arc)
    }
}