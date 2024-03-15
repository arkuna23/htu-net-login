use std::{error::Error, fmt::Display, io};

use crossterm::event;

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

pub enum Signal {
    Action(Action),
    Resize(u16, u16),
    TermEvent(event::Event),
    Error(AppError),
    Exit,
}

pub enum Action {
    Draw,
    Quit,
}

pub enum AppState {
    Load,
    Menu,
    ManageUser,
}

impl Default for AppState {
    fn default() -> Self {
        Self::Load
    }
}
