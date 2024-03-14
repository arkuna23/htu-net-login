pub mod component;
pub mod data;

use std::{
    error::Error, fmt::Display, io::{self, stdout, Stdout}, panic, result
};

use crossterm::{
    event::EnableMouseCapture,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use data::Signal;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

pub(crate) type Result<T> = result::Result<T, TuiError>;
#[derive(Debug)]
pub enum TuiError {

}

impl Display for TuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", "")
    }
}

impl Error for TuiError {}

fn startup() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        shutdown().unwrap();
        hook(info)
    }));

    let _ = stdout().execute(EnableMouseCapture);
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    Ok(terminal)
}

fn shutdown() -> io::Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

pub async fn term_event_loop(tx: UnboundedSender<Signal>) -> JoinHandle<()> {
    
}

pub async fn run() -> io::Result<()> {
    startup()?;
    todo!();
    shutdown()
}
