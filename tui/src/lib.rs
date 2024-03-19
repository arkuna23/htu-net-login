pub mod component;
pub mod data;
pub mod handler;

use std::{
    io::{self, stdout, Stdout},
    panic, result,
};

use component::container::AppContainer;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use data::AppError;
use handler::run_handler;
use ratatui::{backend::CrosstermBackend, Terminal};

pub(crate) type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;
pub(crate) type Result<T> = result::Result<T, AppError>;

fn startup() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        shutdown().unwrap();
        hook(info)
    }));

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    let _ = stdout().execute(EnableMouseCapture);
    Ok(terminal)
}

fn shutdown() -> io::Result<()> {
    let _ = stdout().execute(DisableMouseCapture);
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

pub async fn run(frame_rate: u16, tick_rate: u16) -> Result<()> {
    let terminal = startup().map_err(AppError::StdIo)?;
    run_handler(AppContainer::default(), terminal, frame_rate, tick_rate).await?;
    shutdown().map_err(AppError::StdIo)
}
