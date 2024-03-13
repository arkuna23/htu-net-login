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
    
    let handles = tokio::join!(conf.1, serv_handle);
    handles.0.unwrap().unwrap();
    handles.1.unwrap().unwrap();
}
