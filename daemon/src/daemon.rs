use std::{sync::Arc, time::Duration};

use api::auth::{
    auth_async::{auth, get_auth_info, get_index_page},
    UserInfo,
};
use reqwest::Client;
use tokio::{
    sync::mpsc::{self, Sender},
    task::JoinHandle,
    time,
};

use crate::{config::{self, ConfigFile, ConfigWithLock}, translate::{Chinese, Translation, TranslationKey}, Error};

pub async fn check_autewifi(client: &Client) -> bool {
    if let Ok(resp) = client
        .get("http://192.168.0.1")
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.text())
    {
        resp.await
            .map(|r| r.contains("location.replace(\"http://10."))
            .unwrap_or(false)
    } else {
        false
    }
}

#[derive(Debug)]
pub enum Signal {
    // Exit
}

#[cfg(feature = "sys-notify")]
pub async fn notify(msg: &str) {
    use notify_rust::Notification;

    let result = Notification::new()
        .subtitle("Htu Net Login")
        .summary("Htu Net Login")
        .body(msg)
        .show_async()
        .await;
    if let Err(e) = result {
        eprintln!("sys notify err: {}", e)
    }
}

pub async fn start() -> Result<ConfigFile<ConfigWithLock>, Error> {
    let config = ConfigFile::load_or_create()
        .await?
        .with_lock()
        .await;
    #[cfg(feature = "auto-update")]
    let config = config.with_auto_update().await.map_err(Error::FileNotify)?;
    let config_inner = config.clone();
    tokio::spawn(async {
        let mut fut: Option<JoinHandle<()>> = None;
        let client = Arc::new(Client::new());
        let config = config_inner;
        println!("running daemon");
        loop {
            if fut.as_ref().is_none() || fut.as_ref().unwrap().is_finished() {
                let arc_inner = client.clone();
                let config_inner = config.config().clone();
                fut = Some(tokio::task::spawn_local(async move {
                    if check_autewifi(&arc_inner).await {
                        if let Some(user) = config_inner.lock().await.user() {
                            login(user).await;
                        }
                    };
                    time::sleep(Duration::from_secs(6)).await;
                }));
            }

            time::sleep(Duration::from_millis(250)).await;
        }
    });
    Ok(config)
}

async fn login(user: &UserInfo) -> bool {
    let url = match get_index_page(false).await {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Get index page error: {}", e);
            return false;
        }
    };
    let auth_info = match get_auth_info(&url).await {
        Ok(auth_info) => auth_info,
        Err(e) => {
            eprintln!("Get auth info error: {}", e);
            return false;
        }
    };
    if let Err(e) = auth(url, auth_info, user).await {
        eprintln!("Auth error: {}", e);
        return false;
    };
    println!("connected to htu-net");
    #[cfg(feature = "sys-notify")]
    notify("已连接到校园网").await;
    true
}
