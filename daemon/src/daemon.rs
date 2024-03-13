use std::time::Duration;

use api::auth::{
    auth_async::{auth, get_auth_info, get_index_page},
    UserInfo,
};
use reqwest::Client;
use tokio::{
    runtime::Handle, task::{self, JoinError, JoinHandle, LocalSet}, time
};

use crate::{config::{ConfigFile, ConfigWithLock}, Error};

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

pub async fn start() -> Result<(ConfigFile<ConfigWithLock>, JoinHandle<Result<(), JoinError>>), Error> {
    let config = ConfigFile::load_or_create()
        .await?
        .with_lock()
        .await;
    #[cfg(feature = "auto-update")]
    let config = config.with_auto_update().await.map_err(Error::FileNotify)?;
    let config_inner = config.clone();
    let handle = task::spawn_blocking(move || {
        let rt = Handle::current();
        rt.block_on(async move {
            let client = Client::new();
            let config = config_inner.config().clone();
            let local = LocalSet::new();
            local.run_until(async move {
                println!("running daemon");
                task::spawn_local(async move {
                    loop {
                        if check_autewifi(&client).await {
                            let config = config.clone();
                            if let Some(user) = config.lock().await.user() {
                                login(user).await;
                            };
                        };
            
                        time::sleep(Duration::from_secs(4)).await;
                    }
                }).await
            }).await
        })
    });

    Ok((config, handle))
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
