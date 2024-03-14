pub mod component;

use std::{
    io::{self, stdout, Stdout},
    panic,
};

use crossterm::{
    event::EnableMouseCapture,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};

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

pub async fn run() -> io::Result<()> {
    startup()?;
    todo!();
    shutdown()
}
