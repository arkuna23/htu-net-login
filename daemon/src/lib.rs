pub mod config;
pub mod daemon;
pub mod serve;

use std::{error::Error as StdError, fmt::Display, io};

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    TokioIo(tokio::io::Error),
    StdIo(io::Error),
    #[cfg(feature = "auto-update")]
    FileNotify(notify::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SerdeJson(e) => write!(f, "SerdeJson: {}", e),
            Error::TokioIo(e) => write!(f, "TokioIo: {}", e),
            #[cfg(feature = "auto-update")]
            Error::FileNotify(e) => write!(f, "FileNotify: {}", e),
            Error::StdIo(e) => write!(f, "StdIo: {}", e),
        }
    }
}

impl StdError for Error {}

pub async fn start() {
    let conf = daemon::start().await.unwrap();
    let serv_handle = tokio::spawn(serve::Server::serve(conf.0));

    // !todo pass panic to server thread
    tokio::select! {
        conf_res = conf.1 => {
            if let Err(e) = conf_res {
                log::error!("conf watcher stopped: {:?}", e)
            }
        },
        serv_res = serv_handle => {
            if let Err(e) = serv_res {
                log::error!("server stopped: {:?}", e)
            }
        }
    }
}
