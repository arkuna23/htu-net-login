use std::time::Duration;

use api::auth::{
    auth_async::{auth, get_auth_info, get_index_page},
    AuthError, UserInfo,
};
#[cfg(feature = "sys-notify")]
use notify::Watcher;
use reqwest::ClientBuilder;
use tokio::{
    runtime::Handle,
    task::{self, JoinHandle, LocalSet},
    time,
};

use crate::{
    config::{AppConfig, AppInfo, GlobalAppInfo},
    Error,
};

pub async fn check_autewifi() -> bool {
    let resp = ClientBuilder::new()
        .build()
        .unwrap()
        .get("http://192.168.0.1")
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .map(|r| r.text());
    match resp {
        Ok(resp) => resp
            .await
            .map(|r| r.contains("location.replace(\"http://10."))
            .unwrap_or(false),
        Err(e) => {
            if !e.is_timeout() {
                log::trace!("not autewifi: {:?}", e);
            }
            false
        }
    }
}

#[derive(Debug)]
pub enum Signal {
    // Exit
}

#[cfg(feature = "sys-notify")]
pub async fn notify(msg: &str) {
    use notify_rust::Notification;

    let mut noti = Notification::new();
    let noti = noti.subtitle("Htu Net Login").summary(msg);
    let result_r = {
        #[cfg(target_os = "windows")]
        {
            noti.show()
        }
        #[cfg(target_os = "linux")]
        {
            noti.show_async().await
        }
    };
    log::info!("send notify: {}", msg);
    if let Err(e) = result_r {
        log::error!("sys notify err: {}", e)
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
            let local = LocalSet::new();
            let res = local
                .run_until(async move {
                    log::info!("running login thread");
                    task::spawn_local(async move {
                        let mut success = true;
                        while appinfo.running().await {
                            if !check_autewifi().await {
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
                                    log::info!("login success");
                                }
                                Err(e) => {
                                    let AuthError::AuthFailed { msg } = e else {
                                        log::error!("login error: {}", e);
                                        continue;
                                    };

                                    if success {
                                        #[cfg(feature = "sys-notify")]
                                        notify(&format!("登录失败: {}", msg)).await;
                                        log::error!("login error: {}", msg);
                                        success = false;
                                    }
                                    time::sleep(Duration::from_secs(5)).await;
                                }
                            };
                        }
                        log::info!("login thread exit");
                        #[cfg(feature = "auto-update")]
                        watcher
                            .unwatch(appinfo.read().await.config_path().parent().unwrap())
                            .unwrap()
                    })
                    .await
                })
                .await;
            if let Err(e) = res {
                log::error!("daemon error: {}", e);
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
    log::info!("connected to htu-net");
    #[cfg(feature = "sys-notify")]
    notify("已连接到校园网").await;
    Ok(LoginUrls {
        logout_url_base: logout_url,
        last_url,
    })
}
