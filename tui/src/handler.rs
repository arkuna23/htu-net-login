use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::{sleep, Instant},
};

use crate::{
    component::Component,
    data::{Action, AppError, Signal},
    Result, TuiTerminal,
};

pub fn term_event_loop(
    tx: UnboundedSender<Signal>,
    is_exited: Arc<AtomicBool>,
    tick_rate: u16,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut prev = Instant::now();
        loop {
            if is_exited.load(Ordering::SeqCst) {
                break;
            };
            let has_event = event::poll(
                Duration::from_secs_f64(1f64 / (tick_rate as f64)) - (Instant::now() - prev),
            )
            .unwrap();
            prev = Instant::now();
            if has_event {
                tx.send(term_event_map(event::read().unwrap()).await)
                    .unwrap();
            }
        }
    })
}

async fn term_event_map(event: Event) -> Signal {
    match event {
        Event::Key(key) => {
            if let Some(sig) = keymap(key).await {
                sig
            } else {
                Signal::TermEvent(event)
            }
        }
        Event::Resize(c, r) => Signal::Resize(c, r),
        _ => Signal::TermEvent(event),
    }
}

async fn keymap(event: KeyEvent) -> Option<Signal> {
    match event.code {
        KeyCode::Char('q') => {
            if event.modifiers.contains(KeyModifiers::CONTROL) {
                Some(Signal::Exit)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub async fn run_handler(
    component: impl Component,
    terminal: TuiTerminal,
    frame_rate: u16,
    tick_rate: u16,
) -> Result<()> {
    let (sig_tx, sig_rx) = mpsc::unbounded_channel();
    let exit_mark = Arc::new(AtomicBool::new(false));
    let _term_event = term_event_loop(sig_tx.clone(), exit_mark.clone(), tick_rate);
    run_action_handler(terminal, component, sig_tx, sig_rx, exit_mark, frame_rate).await
}

pub async fn run_action_handler(
    mut terminal: TuiTerminal,
    mut component: impl Component,
    signal_tx: UnboundedSender<Signal>,
    mut signal_rx: UnboundedReceiver<Signal>,
    exit_mark: Arc<AtomicBool>,
    frame_rate: u16,
) -> Result<()> {
    let (act_tx, mut act_rx) = mpsc::unbounded_channel();
    component.init()?;
    component.register_action_sender(act_tx.clone())?;
    let mut prev;
    let mut size = None;
    loop {
        prev = Instant::now();
        while let Ok(e) = signal_rx.try_recv() {
            match e {
                Signal::Exit => {
                    exit_mark.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                Signal::Error(e) => {
                    return Err(e);
                }
                Signal::Resize(c, r) => size = Some((c, r)),
                _ => component.handle_signal(e)?,
            }
        }

        if let Some((c, r)) = size {
            terminal
                .resize(Rect::new(0, 0, c, r))
                .map_err(AppError::StdIo)?;
            act_tx.send(Action::Draw).unwrap();
            size = None;
        }

        while let Ok(action) = act_rx.try_recv() {
            match action {
                Action::Draw => {
                    terminal
                        .draw(|f| {
                            if let Err(e) = component.draw(f, f.size()) {
                                signal_tx.send(Signal::Error(e)).unwrap();
                            }
                        })
                        .map_err(AppError::StdIo)?;
                }
                Action::Quit => signal_tx.send(Signal::Exit).unwrap(),
            };
        }

        sleep(Duration::from_secs_f64(1f64 / (frame_rate as f64)) - (Instant::now() - prev)).await;
    }
}
