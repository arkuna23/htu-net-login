use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use reqwest::Client;
use serde::Serialize;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::{sleep, Instant},
};

use crate::{
    component::Component,
    data::{Action, AppError, DaemonError, DaemonRequest, Signal, UserInfo},
    Result, TuiTerminal,
};

pub async fn term_event_loop(
    tx: UnboundedSender<Event>,
    is_exited: Arc<AtomicBool>,
    tick_rate: u16,
) {
    let duration = Duration::from_secs_f64(1f64 / (tick_rate as f64));
    loop {
        if is_exited.load(Ordering::SeqCst) {
            break;
        };
        let has_event = event::poll(duration).unwrap();
        if has_event {
            tx.send(event::read().unwrap()).unwrap();
        }
    }
}

pub async fn run_handler(
    component: impl Component,
    terminal: TuiTerminal,
    frame_rate: u16,
    tick_rate: u16,
) -> Result<()> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let exit_mark = Arc::new(AtomicBool::new(false));
    tokio::spawn(term_event_loop(event_tx, exit_mark.clone(), tick_rate));
    let (tick_tx, tick_rx) = mpsc::unbounded_channel();
    tokio::spawn(tick_handler(tick_rate, tick_tx, exit_mark.clone()));
    run_action_handler(
        terminal, component, tick_rx, event_rx, exit_mark, frame_rate,
    )
    .await
}

async fn tick_handler(tick_rate: u16, tx: UnboundedSender<()>, exit_mark: Arc<AtomicBool>) {
    let duration = Duration::from_secs_f64(1f64 / (tick_rate as f64));
    while !exit_mark.load(Ordering::SeqCst) {
        tx.send(()).unwrap();
        sleep(duration).await;
    }
}

async fn run_action_handler(
    mut terminal: TuiTerminal,
    mut component: impl Component,
    mut tick_rx: UnboundedReceiver<()>,
    mut event_rx: UnboundedReceiver<Event>,
    exit_mark: Arc<AtomicBool>,
    frame_rate: u16,
) -> Result<()> {
    let (signal_tx, mut signal_rx) = mpsc::unbounded_channel::<Signal>();
    let (act_tx, mut act_rx) = mpsc::unbounded_channel();
    let _info = component.init()?;
    component.register_action_sender(act_tx.clone())?;
    let mut prev;
    let mut size = None;
    loop {
        prev = Instant::now();
        while let Ok(e) = signal_rx.try_recv() {
            match e {
                Signal::Exit => {
                    component.handle_signal(e)?;
                    exit_mark.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                _ => component.handle_signal(e)?,
            }
        }

        while let Ok(e) = event_rx.try_recv() {
            match e {
                Event::Key(key) => {
                    if let Some(signal) = handle_key(key).await {
                        signal_tx.send(signal).unwrap();
                    } else if _info.key_enabled {
                        component.handle_key(key)?;
                    }
                }
                Event::Resize(c, r) => {
                    size = Some((c, r));
                }
                Event::Mouse(mouse) if _info.mouse_enabled => {
                    component.handle_mouse(mouse)?;
                }
                _ => {}
            }
        }

        if tick_rx.try_recv().is_ok() {
            component.tick()?;
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
                                signal_tx.send(Signal::DrawError(e)).unwrap();
                            }
                        })
                        .map_err(AppError::StdIo)?;
                }
                Action::Quit => signal_tx.send(Signal::Exit).unwrap(),
                Action::PingDaemon => {
                    let signal_tx = signal_tx.clone();
                    tokio::spawn(async move {
                        if let Ok(resp) = reqwest::get("http://127.0.0.1:11451/").await {
                            if resp.status().is_success() {
                                signal_tx.send(Signal::DaemonPong).unwrap();
                            }
                        }
                    });
                }
                Action::GetAccount => {
                    let signal_tx = signal_tx.clone();
                    tokio::spawn(async move {
                        if let Ok(resp) = reqwest::get("http://127.0.0.1:11451/user").await {
                            if let Ok(user) = resp.json::<UserInfo>().await {
                                signal_tx.send(Signal::UserInfo(user)).unwrap();
                            }
                        }
                    });
                }
                Action::SelectInput(id) => signal_tx.send(Signal::InputSelected(id)).unwrap(),
                Action::SelectCheckbox(id) => signal_tx.send(Signal::CheckboxSelected(id)).unwrap(),
                Action::SetAccount(user) => {
                    send_daemon_request(
                        "http://127.0.0.1:11451/user",
                        Some(user),
                        signal_tx.clone(),
                        DaemonRequest::SetAccount,
                        true,
                    )
                    .await
                }
                Action::JumpTo(page) => signal_tx.send(Signal::ChangePage(page)).unwrap(),
                Action::Logout => {
                    send_daemon_request::<UserInfo>(
                        "http://127.0.0.1:11451/logout",
                        None,
                        signal_tx.clone(),
                        DaemonRequest::Logout,
                        false,
                    )
                    .await;
                }
            };
        }

        sleep(Duration::from_secs_f64(1f64 / (frame_rate as f64)) - (Instant::now() - prev)).await;
    }
}

async fn handle_key(key: KeyEvent) -> Option<Signal> {
    if key.modifiers == KeyModifiers::CONTROL {
        match key.code {
            KeyCode::Char('q') => Some(Signal::Exit),
            _ => None,
        }
    } else {
        None
    }
}

async fn send_daemon_request<S: Serialize + Send + Sync + 'static>(
    url: &str,
    json: Option<S>,
    signal_tx: UnboundedSender<Signal>,
    req_type: DaemonRequest,
    post: bool,
) {
    let url = url.to_owned();
    tokio::spawn(async move {
        let client = Client::default();
        let res = if post {
            match json {
                Some(j) => client.post(url).json(&j),
                None => client.post(url),
            }
        } else {
            client.get(url)
        }
        .send()
        .await;
        match res {
            Ok(resp) => {
                if resp.status().is_success() {
                    signal_tx
                        .send(Signal::DaemonResponse {
                            req: req_type,
                            result: Ok(()),
                        })
                        .unwrap();
                } else {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => signal_tx
                            .send(Signal::DaemonResponse {
                                req: req_type,
                                result: Err(DaemonError::ErrMessage(json)),
                            })
                            .unwrap(),
                        Err(e) => signal_tx
                            .send(Signal::DaemonResponse {
                                req: req_type,
                                result: Err(DaemonError::Reqwest(e)),
                            })
                            .unwrap(),
                    };
                }
            }
            Err(e) => signal_tx
                .send(Signal::DaemonResponse {
                    req: req_type,
                    result: Err(DaemonError::Reqwest(e)),
                })
                .unwrap(),
        };
    });
}
