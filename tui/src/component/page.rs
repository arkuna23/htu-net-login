use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::Stylize,
    text::Line,
};
use tokio::{
    sync::mpsc::UnboundedSender,
    time::{self, Instant},
};

use crate::data::{Action, Signal};

use super::{form::UserForm, Component, ComponentInfo};

#[derive(Default)]
pub struct Page {
    action_tx: Option<UnboundedSender<Action>>,
    inner_info: ComponentInfo,
    inner: Box<dyn Component>,
    last_pong: Option<Instant>,
    mouse_area: Rect,
    exit_state: Arc<AtomicBool>,
    connected: bool,
}

impl Component for Page {
    fn init(&mut self) -> crate::Result<super::ComponentInfo> {
        let mut form = UserForm::default();
        self.inner_info = form.init()?;
        self.inner = Box::new(form);
        Ok(ComponentInfo::all_enabled())
    }

    fn register_action_sender(&mut self, sender: UnboundedSender<Action>) -> crate::Result<()> {
        let tx = sender.clone();
        self.inner.register_action_sender(sender.clone())?;
        self.action_tx = Some(sender);
        let state = self.exit_state.clone();
        tokio::spawn(async move {
            while !state.load(Ordering::SeqCst) {
                tx.send(Action::PingDaemon).unwrap();
                time::sleep(Duration::from_secs(1)).await;
            }
        });
        Ok(())
    }

    fn draw(&mut self, f: &mut ratatui::prelude::Frame, rect: Rect) -> crate::Result<()> {
        self.mouse_area = rect;
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(rect);
        f.render_widget(
            Line::from(vec!["守护进程".light_cyan(), " ".into(), {
                if self.connected {
                    "可用".bold().green()
                } else {
                    "不可用".bold().red()
                }
            }])
            .centered(),
            layout[1],
        );
        self.inner.draw(f, layout[0])?;

        Ok(())
    }

    fn handle_signal(&mut self, signal: crate::data::Signal) -> crate::Result<()> {
        match signal {
            Signal::DaemonPong => {
                self.last_pong = Some(Instant::now());
            }
            Signal::Exit => {
                self.exit_state
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            _ => (),
        };
        self.inner.handle_signal(signal)?;

        Ok(())
    }

    fn tick(&mut self) -> crate::Result<()> {
        let mut new_state = false;
        if let Some(last_pong) = self.last_pong {
            if (Instant::now() - last_pong).as_secs() < 3 {
                new_state = true;
            }
        }

        if new_state != self.connected {
            self.connected = new_state;
            self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
        }

        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::Result<()> {
        if self.inner_info.key_enabled {
            self.inner.handle_key(key)?;
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        if self.inner_info.mouse_enabled
            && self
                .inner
                .mouse_area()
                .contains(Position::new(mouse.column, mouse.row))
        {
            self.inner.handle_mouse(mouse)?;
        }

        Ok(())
    }
}
