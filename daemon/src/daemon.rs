use std::time::Duration;

use api::auth::{
    auth_async::{auth, get_auth_info, get_index_page},
    AuthError, UserInfo,
};
use notify::Watcher;
use reqwest::Client;
use tokio::{
    runtime::Handle,
    task::{self, JoinHandle, LocalSet},
    time,
};

use crate::{
    config::{AppConfig, AppInfo, GlobalAppInfo},
    Error,
};

pub async fn check_autewifi(client: &Client) -> bool {
    if let Ok(resp) = client
        .get("http://192.168.0.1")
        .timeout(Duration::from_secs(1))
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
    println!("send notify: {}", msg);
    if let Err(e) = result {
        eprintln!("sys notify err: {}", e)
    }
}

pub async fn start() -> Result<(GlobalAppInfo, JoinHandle<()>), Error> {
    let appinfo = AppInfo::load_or_create().await?.global();
    #[cfg(feature = "auto-update")]
    let (mut watcher, notify_handle) =
        appinfo.run_auto_update().await.map_err(Error::FileNotify)?;

    let appinfo_inner = appinfo.clone();
    let handle = task::spawn_blocking(move || {
        let appinfo = appinfo_inner;
        let rt = Handle::current();
        rt.block_on(async move {
            let client = Client::new();
            let local = LocalSet::new();
            let res = local
                .run_until(async move {
                    println!("running login thread");
                    task::spawn_local(async move {
                        let mut success = true;
                        while appinfo.running().await {
                            if !check_autewifi(&client).await {
                                continue;
                            }

                            let user = appinfo.read().await.config().user().cloned();
                            let Some(user) = user else {
                                time::sleep(Duration::from_secs(5)).await;
                                continue;
                            };
                            match login(user).await {
                                Ok(url) => {
                                    success = true;
                                    let mut appinfo_write = appinfo.write().await;
                                    appinfo_write.config_mut().set_last_url(url.last_url);
                                    appinfo_write
                                        .config_mut()
                                        .set_logout_url_base(url.logout_url_base);
                                    drop(appinfo_write);
                                    let _ = appinfo.read().await.save().await;
                                    println!("login success");
                                }
                                Err(e) => {
                                    let AuthError::AuthFailed { msg } = e else {
                                        eprintln!("login error: {}", e);
                                        continue;
                                    };

                                    if success {
                                        #[cfg(feature = "sys-notify")]
                                        notify(&format!("登录失败: {}", msg)).await;
                                        eprintln!("login error: {}", msg);
                                        success = false;
                                    }
                                    time::sleep(Duration::from_secs(5)).await;
                                }
                            };
                        }
                        println!("login thread exit");
                        #[cfg(feature = "auto-update")]
                        watcher
                            .unwatch(appinfo.read().await.config_path().parent().unwrap())
                            .unwrap()
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
    return Ok((appinfo, handle));
    #[cfg(feature = "auto-update")]
    Ok((
        appinfo,
        tokio::spawn(async move {
            tokio::try_join!(handle, notify_handle).unwrap();
        }),
    ))
}

pub async fn login_net(user: &UserInfo) -> Result<(), AuthError> {
    let url = get_index_page(false).await?;
    let auth_info = get_auth_info(&url).await?;
    auth(url, auth_info, user).await
}

struct LoginUrls {
    logout_url_base: String,
    last_url: String,
}

async fn login(user: UserInfo) -> Result<LoginUrls, AuthError> {
    let url = get_index_page(false).await?;
    let auth_info = get_auth_info(&url).await?;
    let last_url = url.url.clone();
    let logout_url = auth_info.logout_url_root.clone();
    auth(url, auth_info, &user).await?;
    println!("connected to htu-net");
    #[cfg(feature = "sys-notify")]
    notify("已连接到校园网").await;
    Ok(LoginUrls {
        logout_url_base: logout_url,
        last_url,
    })
}
