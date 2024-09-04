use std::path::Path;

use winapi::um::{
    processthreadsapi::OpenProcessToken,
    wincon::FreeConsole,
    winnt::{HANDLE, TOKEN_ELEVATION, TOKEN_QUERY},
};
pub static BIN_PATH: &str = "C:\\Windows\\htu-net.exe";

fn free_console() -> bool {
    unsafe { FreeConsole() == 0 }
}

#[cfg(feature = "daemon")]
pub mod daemon {
    use std::{io, time::Duration};

    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

    use crate::service::init_logger;

    use super::BIN_PATH;

    pub static DAEMON_NAME: &str = "HtuNet";

    fn get_regkey() -> io::Result<winreg::RegKey> {
        winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_WRITE | KEY_READ,
        )
    }

    pub async fn install_daemon() -> std::io::Result<()> {
        get_regkey()?.set_value(DAEMON_NAME, &format!("{} -d -b", BIN_PATH))
    }

    pub async fn uninstall_daemon() -> anyhow::Result<()> {
        get_regkey()?.delete_value(DAEMON_NAME)?;
        let _ = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(1))
            .build()?
            .get("http://127.0.0.1:11451/exit")
            .send()
            .await;
        Ok(())
    }

    pub fn is_daemon_installed() -> io::Result<bool> {
        Ok(get_regkey()?.get_value::<String, _>(DAEMON_NAME).is_ok())
    }

    pub async fn run_service(background: bool) -> anyhow::Result<()> {
        if background {
            super::free_console();
        }
        init_logger()?;
        htu_net_login_daemon::start().await;
        Ok(())
    }
}

pub fn is_installed() -> std::io::Result<bool> {
    let mut installed = Path::new(BIN_PATH).exists();
    #[cfg(feature = "daemon")]
    {
        installed = installed && daemon::is_daemon_installed()?;
    }
    Ok(installed)
}

#[inline]
pub fn is_bin_exists() -> bool {
    Path::new(BIN_PATH).exists()
}

pub async fn install_bin() -> anyhow::Result<()> {
    let current_exe_path = std::env::current_exe()?;
    tokio::fs::copy(current_exe_path, BIN_PATH).await?;
    Ok(())
}

pub fn is_admin() -> bool {
    unsafe {
        let mut htoken: HANDLE = std::ptr::null_mut();
        let result = OpenProcessToken(
            winapi::um::processthreadsapi::GetCurrentProcess(),
            TOKEN_QUERY,
            &mut htoken,
        );

        if result == 0 {
            return false;
        }

        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut return_length: u32 = 0;

        let result = winapi::um::securitybaseapi::GetTokenInformation(
            htoken,
            winapi::um::winnt::TokenElevation,
            &mut elevation as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        if result == 0 {
            return false;
        }

        elevation.TokenIsElevated != 0
    }
}
