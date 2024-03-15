use std::time::Duration;

use api::auth::auth_async::{auth, get_auth_info, get_index_page};
use reqwest::Client;
use tokio::{
    runtime::Handle,
    task::{self, JoinError, JoinHandle, LocalSet},
    time,
};

use crate::{
    config::{AppConfig, ConfigWithLock},
    Error,
};

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

pub async fn start() -> Result<
    (
        AppConfig<ConfigWithLock>,
        JoinHandle<Result<((), (), ()), JoinError>>,
    ),
    Error,
> {
    let config = AppConfig::load_or_create().await?.with_lock().await;
    #[cfg(feature = "auto-update")]
    let (config, sender_handle, recv_handle) =
        config.with_auto_update().await.map_err(Error::FileNotify)?;
    let config_inner = config.clone();
    let handle = task::spawn_blocking(move || {
        let rt = Handle::current();
        rt.block_on(async move {
            let client = Client::new();
            let local = LocalSet::new();
            let res = local
                .run_until(async move {
                    println!("running login thread");
                    task::spawn_local(async move {
                        while config_inner.running() {
                            if check_autewifi(&client).await {
                                if config_inner.config().read().await.user().is_some() {
                                    login(config_inner.clone()).await;
                                } else {
                                    time::sleep(Duration::from_secs(1)).await;
                                };
                            };
                        }
                        println!("login thread exit");
                    })
                    .await
                })
                .await;
            if let Err(e) = res {
                eprintln!("daemon error: {}", e);
            }
        })
    });

    #[cfg(not(feature = "auto-update"))]
    return Ok((config, handle));
    #[cfg(feature = "auto-update")]
    Ok((
        config,
        tokio::spawn(async move { tokio::try_join!(sender_handle, recv_handle, handle) }),
    ))
}

async fn login(app_config: AppConfig<ConfigWithLock>) -> bool {
    let url = match get_index_page(false).await {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Get index page error: {}", e);
            return false;
        }
    };
    app_config.config().write().await.set_last_url(&url.url);
    let _ = app_config.save().await;
    let auth_info = match get_auth_info(&url).await {
        Ok(auth_info) => auth_info,
        Err(e) => {
            eprintln!("Get auth info error: {}", e);
            return false;
        }
    };
    if let Err(e) = auth(
        url,
        auth_info,
        app_config.config().read().await.user().unwrap(),
    )
    .await
    {
        eprintln!("Auth error: {}", e);
        return false;
    };
    println!("connected to htu-net");
    #[cfg(feature = "sys-notify")]
    notify("已连接到校园网").await;
    true
}
