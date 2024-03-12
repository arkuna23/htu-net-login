pub mod config;
pub mod daemon;
pub mod serve;

use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    TokioIo(tokio::io::Error),
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
        }
    }
}

impl StdError for Error {}

pub async fn start() {
    let conf = daemon::start().await.unwrap();
    serve::Server::serve(conf).await.unwrap();
}
