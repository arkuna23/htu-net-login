#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod serv;

use flexi_logger::{Duplicate, FileSpec, Logger, LoggerHandle, WriteMode};
use htu_net_login_daemon::config::config_dir;
#[cfg(feature = "daemon")]
pub use serv::daemon::{install_daemon, run_service, uninstall_daemon};
#[allow(unused_imports)]
pub use serv::{install_bin, is_admin, is_bin_exists, is_installed, BIN_PATH};

pub fn init_logger() -> anyhow::Result<LoggerHandle> {
    Ok(Logger::try_with_str("debug, reqwest::connect=info")?
        .log_to_file(
            FileSpec::default().directory(config_dir().ok_or(anyhow::anyhow!("empty config dir"))?),
        )
        .duplicate_to_stdout(Duplicate::Debug)
        .duplicate_to_stderr(Duplicate::Warn)
        .write_mode(WriteMode::Direct)
        .start()?)
}
