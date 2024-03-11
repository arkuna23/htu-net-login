pub mod config;
pub mod daemon;
pub mod serve;

use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    TokioIo(tokio::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SerdeJson(e) => write!(f, "SerdeJson: {}", e),
            Error::TokioIo(e) => write!(f, "TokioIo: {}", e),
        }
    }
}

impl StdError for Error {}
