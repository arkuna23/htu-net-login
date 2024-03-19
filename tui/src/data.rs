use std::{error::Error, fmt::Display, io};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum AppError {
    StdIo(io::Error),
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdIo(e) => write!(f, "std io error: {}", e),
        }
    }
}

impl Error for AppError {}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserInfo {
    pub id: String,
    pub password: String,
    pub suffix: Suffix,
}

#[derive(Clone, Copy, Debug)]
pub enum Suffix {
    ChinaMobie,
    ChinaUnicom,
    ChinaTelecom,
    Local,
}

impl Default for Suffix {
    fn default() -> Self {
        Self::Local
    }
}

impl Serialize for Suffix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_str())
    }
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

impl<'de> Deserialize<'de> for Suffix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            Self::CM => Ok(Self::ChinaMobie),
            Self::CU => Ok(Self::ChinaUnicom),
            Self::CT => Ok(Self::ChinaTelecom),
            Self::LOCAL => Ok(Self::Local),
            _ => Err(serde::de::Error::custom("Invalid Suffix")),
        }
    }
}

impl ToString for Suffix {
    fn to_string(&self) -> String {
        self.to_str().to_string()
    }
}

pub enum Signal {
    DrawError(AppError),
    UserInfo(UserInfo),
    InputSelected(u16),
    DaemonPong,
    Exit,
}

pub enum Action {
    PingDaemon,
    Draw,
    Quit,
    GetUser,
    SelectInput(u16),
}

pub enum AppState {
    Load,
    Menu,
    ManageUser,
}

impl Default for AppState {
    fn default() -> Self {
        Self::ManageUser
    }
}
