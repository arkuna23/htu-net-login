use core::fmt;
use std::fmt::{Debug, Display, Formatter};

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref JS_URL_PATTERN: Regex =
        Regex::new(r#"<script.*?src="(.*?js/common.js).*?".*?>"#).unwrap();
}

#[derive(Debug)]
pub struct IndexUrl {
    pub url: String,
    pub root: String,
    pub args: Vec<(String, String)>,
}

fn get_root_url(url: &str) -> String {
    let start = url.find("://").unwrap() + 3;
    let end = url[start..].find('/').unwrap() + start;
    url[..end].to_string()
}

fn parse_index_page(html: &str) -> Option<IndexUrl> {
    let url = {
        let start = html.find('"')? + 1;
        let end = html[start..].find('"')? + start;
        html[start..end].to_string()
    };
    let root_url = get_root_url(&url);
    let args = {
        let start = url.find('?')? + 1;
        url::form_urlencoded::parse(url[start..].as_bytes())
            .into_owned()
            .collect()
    };
    Some(IndexUrl {
        url,
        root: root_url,
        args,
    })
}

fn get_js_url(html: &str) -> Option<&str> {
    let caps = JS_URL_PATTERN.captures(html)?;
    let url = caps.get(1)?.as_str();
    Some(url)
}

fn get_variable_value<'a>(js_code: &'a str, variable_name: &str) -> Option<&'a str> {
    let assignment = format!("{} =", variable_name);
    if let Some(start_index) = js_code.find(&assignment) {
        let start_index = start_index + assignment.len();
        let remaining_code = &js_code[start_index..];
        if let Some(end_index) = remaining_code.find(';') {
            let value = &remaining_code[..end_index];
            let trimmed_value = value.trim();

            if trimmed_value.starts_with('\'') || trimmed_value.starts_with('"') {
                let extracted_value = &trimmed_value[1..trimmed_value.len() - 1];

                #[cfg(debug_assertions)]
                println!("{} = {}", variable_name, extracted_value);
                return Some(extracted_value);
            }
        }
    }

    None
}

pub struct AuthInfo {
    pub logout_url_root: String,
    pub auth_url: String,
    pub school_codes: String,
}

fn get_js_auth_info(js: &str) -> AuthInfo {
    let auth_url = get_variable_value(js, "authApiUrl").unwrap();
    let logout_root = get_root_url(auth_url).split(':').collect::<Vec<&str>>()[0].to_string();
    AuthInfo {
        logout_url_root: logout_root,
        auth_url: auth_url.into(),
        school_codes: get_variable_value(js, "authSchoolCodes")
            .unwrap()
            .to_string(),
    }
}

#[derive(Serialize, Deserialize)]
pub enum Suffix {
    ChinaMobie,
    ChinaUnicom,
    ChinaTelecom,
    Local,
}

impl Suffix {
    const CM: &'static str = "@yd";
    const CU: &'static str = "@lt";
    const CT: &'static str = "@dx";
    const LOCAL: &'static str = "@hsd";

    pub fn to_str(&self) -> &str {
        match self {
            Suffix::ChinaMobie => Self::CM,
            Suffix::ChinaUnicom => Self::CU,
            Suffix::ChinaTelecom => Self::CT,
            Suffix::Local => Self::LOCAL,
        }
    }
}

impl Debug for Suffix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChinaMobie => write!(f, "ChinaMobie"),
            Self::ChinaUnicom => write!(f, "ChinaUnicom"),
            Self::ChinaTelecom => write!(f, "ChinaTelecom"),
            Self::Local => write!(f, "Local"),
        }
    }
}

impl ToString for Suffix {
    fn to_string(&self) -> String {
        self.to_str().to_string()
    }
}

#[derive(Debug)]
pub enum AuthError<T> {
    ReqError(reqwest::Error),
    InvalidResponse(T),
    AuthFailed { msg: String },
    Authed,
}

impl<T: Display> Display for AuthError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::ReqError(e) => write!(f, "Request Error: {}", e),
            AuthError::InvalidResponse(e) => write!(f, "Invalid Response: {}", e),
            AuthError::Authed => write!(f, "Already Authenticated"),
            AuthError::AuthFailed { msg } => write!(f, "Authentication Failed: {}", msg),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    id: String,
    password: String,
    suffix: Suffix,
}

#[cfg(feature = "async")]
pub mod auth_async {
    use reqwest::Client;

    use crate::tool::ping_async;

    use super::*;

    pub async fn get_index_page(ping: bool) -> Result<IndexUrl, AuthError<String>> {
        if ping && ping_async("www.baidu.com", 80).await.is_ok() {
            return Err(AuthError::Authed);
        }

        let resp = reqwest::get("http://192.168.0.1")
            .await
            .map_err(AuthError::ReqError)?
            .text()
            .await
            .map_err(AuthError::ReqError)?;
        parse_index_page(&resp).ok_or(AuthError::InvalidResponse(resp))
    }

    pub async fn get_auth_info(index_url: &IndexUrl) -> Result<AuthInfo, AuthError<String>> {
        let resp = reqwest::get(index_url.url.clone())
            .await
            .map_err(AuthError::ReqError)?
            .text()
            .await
            .map_err(AuthError::ReqError)?;
        let js_url = get_js_url(&resp).ok_or_else(|| AuthError::InvalidResponse(resp.clone()))?;
        #[cfg(debug_assertions)]
        println!("JS URL: {}", js_url);

        Ok(get_js_auth_info(
            reqwest::get(index_url.root.clone() + js_url)
                .await
                .map_err(AuthError::ReqError)?
                .text()
                .await
                .map_err(AuthError::ReqError)?
                .as_str(),
        ))
    }

    pub async fn auth(
        index_url: IndexUrl,
        auth_info: AuthInfo,
        user: UserInfo,
    ) -> Result<(), AuthError<serde_json::Value>> {
        let client = Client::new();
        // first auth
        let resp = client
            .post(auth_info.auth_url)
            .form(&[
                ("campusCode", auth_info.school_codes.as_str()),
                ("username", user.id.as_str()),
                ("password", user.password.as_str()),
                ("operatorSuffix", user.suffix.to_str()),
            ])
            .send()
            .await
            .map_err(AuthError::ReqError)?
            .json::<serde_json::Value>()
            .await
            .map_err(AuthError::ReqError)?;
        #[cfg(debug_assertions)]
        println!("{:?}", resp);

        if let Some(code) = resp.get("code").and_then(|r| r.as_i64()) {
            if code != 1 {
                return Err(AuthError::AuthFailed {
                    msg: resp
                        .get("msg")
                        .and_then(|r| r.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                });
            }
        } else {
            return Err(AuthError::InvalidResponse(resp));
        }

        // quick auth
        let resp = client
            .get(
                index_url.root
                    + "/quickauth.do?"
                    + url::form_urlencoded::Serializer::new(&mut String::new())
                        .extend_pairs(index_url.args)
                        .append_pair("userid", (user.id + user.suffix.to_str()).as_str())
                        .append_pair("passwd", user.password.as_str())
                        .finish(),
            )
            .send()
            .await
            .map_err(AuthError::ReqError)?
            .json::<serde_json::Value>()
            .await
            .map_err(AuthError::ReqError)?;
        #[cfg(debug_assertions)]
        println!("{:?}", resp);
        if let Some(str) = resp.get("code").and_then(|r| r.as_str()) {
            if str == "0" {
                Ok(())
            } else {
                return Err(AuthError::AuthFailed {
                    msg: resp
                        .get("message")
                        .and_then(|r| r.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                });
            }
        } else {
            Err(AuthError::InvalidResponse(resp))
        }
    }

    mod tests {
        use tokio::test;

        #[allow(unused_imports)]
        use crate::auth::parse_index_page;
        #[allow(unused_imports)]
        use crate::logout::logout_async;

        #[test]
        async fn get_index_page_test() {
            use super::get_index_page;
            logout_async("http://10.101.2.205").await.ok();
            println!("{:?}", get_index_page(true).await.unwrap());
        }

        #[test]
        async fn auth_test() {
            logout_async("http://10.101.2.205").await.unwrap();
            let index_url = super::get_index_page(true).await.unwrap();
            println!("{:?}", index_url);
            let auth_info = super::get_auth_info(&index_url).await.unwrap();
            let user = super::UserInfo {
                id: "".to_string(),
                password: "".to_string(),
                suffix: super::Suffix::ChinaMobie,
            };
            super::auth(index_url, auth_info, user).await.unwrap();
        }
    }
}
#[cfg(feature = "blocking")]
mod auth_blocking {

    use super::*;

    pub fn get_index_page(ping_check: bool) -> Result<IndexUrl, AuthError<String>> {
        use crate::tool::ping;

        if ping_check && ping("www.baidu.com", 80).is_ok() {
            return Err(AuthError::Authed);
        }
        let resp = reqwest::blocking::get("http://192.168.0.1")
            .map_err(AuthError::ReqError)?
            .text()
            .map_err(AuthError::ReqError)?;

        parse_index_page(&resp).ok_or(AuthError::InvalidResponse(resp))
    }

    pub fn get_auth_info(index_url: &IndexUrl) -> Result<AuthInfo, AuthError<String>> {
        let resp = reqwest::blocking::get(index_url.url.clone())
            .map_err(AuthError::ReqError)?
            .text()
            .map_err(AuthError::ReqError)?;
        let js_url = get_js_url(&resp).ok_or_else(|| AuthError::InvalidResponse(resp.clone()))?;
        #[cfg(debug_assertions)]
        println!("JS URL: {}", js_url);
        Ok(get_js_auth_info(
            reqwest::blocking::get(index_url.root.clone() + js_url)
                .map_err(AuthError::ReqError)?
                .text()
                .map_err(AuthError::ReqError)?
                .as_str(),
        ))
    }

    //blocking auth function
    pub fn auth(
        index_url: IndexUrl,
        auth_info: AuthInfo,
        user: UserInfo,
    ) -> Result<(), AuthError<serde_json::Value>> {
        let client = reqwest::blocking::Client::new();
        // first auth
        let resp = client
            .post(auth_info.auth_url)
            .form(&[
                ("campusCode", auth_info.school_codes.as_str()),
                ("username", user.id.as_str()),
                ("password", user.password.as_str()),
                ("operatorSuffix", user.suffix.to_str()),
            ])
            .send()
            .map_err(AuthError::ReqError)?
            .json::<serde_json::Value>()
            .map_err(AuthError::ReqError)?;
        #[cfg(debug_assertions)]
        println!("{:?}", resp);
        if let Some(code) = resp.get("code").and_then(|r| r.as_i64()) {
            if code != 1 {
                return Err(AuthError::AuthFailed {
                    msg: resp
                        .get("msg")
                        .and_then(|r| r.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                });
            }
        } else {
            return Err(AuthError::InvalidResponse(resp));
        }
        // quick auth
        let resp = client
            .get(
                index_url.root
                    + "/quickauth.do?"
                    + url::form_urlencoded::Serializer::new(&mut String::new())
                        .extend_pairs(index_url.args)
                        .append_pair("userid", (user.id + user.suffix.to_str()).as_str())
                        .append_pair("passwd", user.password.as_str())
                        .finish(),
            )
            .send()
            .map_err(AuthError::ReqError)?
            .json::<serde_json::Value>()
            .map_err(AuthError::ReqError)?;
        #[cfg(debug_assertions)]
        println!("{:?}", resp);
        if let Some(str) = resp.get("code").and_then(|r| r.as_str()) {
            if str == "0" {
                Ok(())
            } else {
                Err(AuthError::AuthFailed {
                    msg: resp
                        .get("message")
                        .and_then(|r| r.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                })
            }
        } else {
            Err(AuthError::InvalidResponse(resp))
        }
    }

    // blocking tests
    mod tests {

        #[test]
        fn get_index_page_test() {
            use crate::auth::auth_blocking::get_index_page;
            use crate::logout::logout;
            logout("http://10.101.2.205").ok();
            get_index_page(false).unwrap();
        }

        #[test]
        fn auth_test() {
            use crate::auth::auth_blocking::{auth, get_auth_info, get_index_page};
            use crate::logout::logout;
            logout("http://10.101.2.205").ok();
            let index_url = get_index_page(false).unwrap();
            println!("{:?}", index_url);
            let auth_info = get_auth_info(&index_url).unwrap();
            let user = super::UserInfo {
                id: "".to_string(),
                password: "".to_string(),
                suffix: super::Suffix::ChinaMobie,
            };
            auth(index_url, auth_info, user).unwrap();
        }
    }
}
