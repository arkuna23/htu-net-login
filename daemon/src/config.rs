use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
};

use api::auth::UserInfo;
use dirs::config_dir;
use notify::{
    event::{self, DataChange, ModifyKind},
    RecommendedWatcher, Watcher,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io,
    sync::{self, Mutex},
};

use crate::Error;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    user: Option<UserInfo>,
}

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
                config: serde_json::from_slice(&fs::read(&path).await.map_err(Error::TokioIo)?)
                    .map_err(Error::SerdeJson)?,
                path,
            })
        }
    }

    pub async fn save(&self) -> io::Result<()> {
        let (path, _) = Self::get_or_create_path().await?;
        fs::write(path, serde_json::to_vec(&self.config)?).await
    }
}

#[cfg(feature = "auto-update")]
impl Config {
    async fn run_auto_update<P: ToOwned<Owned = PathBuf>>(
        self,
        path: P,
    ) -> notify::Result<Arc<Mutex<Self>>> {
        let arc = Arc::new(Mutex::new(self));
        let conf_path = path.to_owned();
        let (tx, rx) = mpsc::channel();
        let (async_tx, mut async_rx) = sync::mpsc::channel::<notify::Event>(4);
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;
        watcher.watch(&conf_path, notify::RecursiveMode::NonRecursive)?;
        thread::spawn(move || {
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
                if let notify::EventKind::Modify(ModifyKind::Data(_)) = event.kind {
                    if let Ok(data) = fs::read(&conf_path).await {
                        match serde_json::from_slice(&data) {
                            Ok(conf) => {
                                let mut config = arc_inner.lock().await;
                                *config = conf;
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
