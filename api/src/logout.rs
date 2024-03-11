use reqwest::Error as ReqError;
use serde_json::Value;
use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum LogoutError {
    JSON(Value),
    Request(ReqError),
}

impl Display for LogoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSON(v) => {
                write!(f, "{}", v)
            }
            Self::Request(e) => {
                write!(f, "{}", e)
            }
        }
    }
}

impl Error for LogoutError {}

fn parse_result(response: Value) -> Result<(), LogoutError> {
    if let Some(1) = response.get("result").and_then(|r| r.as_i64()) {
        Ok(())
    } else {
        Err(LogoutError::JSON(response))
    }
}

#[cfg(feature = "blocking")]
pub fn logout(base_url: &str) -> Result<(), LogoutError> {
    let client = reqwest::blocking::Client::new();
    let res: Value = client
        .post(format!("{}/loginOut", base_url))
        .send()
        .map_err(LogoutError::Request)?
        .json()
        .map_err(LogoutError::Request)?;
    parse_result(res)
}

#[cfg(feature = "async")]
pub async fn logout_async(base_url: &str) -> Result<(), LogoutError> {
    let client = reqwest::Client::new();
    let res: Value = client
        .post(format!("{}/loginOut", base_url))
        .send()
        .await
        .map_err(LogoutError::Request)?
        .json()
        .await
        .map_err(LogoutError::Request)?;
    parse_result(res)
}

mod tests {
    #[test]
    #[cfg(feature = "blocking")]
    fn test_logout() {
        super::logout("http://10.101.2.205").unwrap();
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_logout_async() {
        super::logout_async("http://10.101.2.205").await.unwrap();
    }
}
