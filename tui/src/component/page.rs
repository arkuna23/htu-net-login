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

use crate::data::{Action, AppState, Signal};

use super::{form::UserForm, Component, ComponentInfo};

#[derive(Default)]
pub struct Page {
    action_tx: Option<UnboundedSender<Action>>,
    app_state: AppState,
    user_form: Option<UserForm>,
    last_pong: Option<Instant>,
    mouse_area: Rect,
    exit_state: Arc<AtomicBool>,
    connected: bool,
}

impl Component for Page {
    fn init(&mut self) -> crate::Result<super::ComponentInfo> {
        let mut form = UserForm::default();
        form.init()?;
        self.user_form = Some(form);
        Ok(ComponentInfo::all_enabled())
    }

    fn register_action_sender(&mut self, sender: UnboundedSender<Action>) -> crate::Result<()> {
        let tx = sender.clone();
        #[cfg(debug_assertions)]
        self.user_form
            .as_mut()
            .unwrap()
            .register_action_sender(sender.clone())?;
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

        match self.app_state {
            AppState::ManageUser => self.user_form.as_mut().unwrap().draw(f, layout[0])?,
            _ => (),
        };

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
            Signal::InputSelected(_) | Signal::UserInfo(_) => {
                self.user_form.as_mut().unwrap().handle_signal(signal)?;
            }
            _ => (),
        };

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
        match self.app_state {
            AppState::ManageUser => self.user_form.as_mut().unwrap().handle_key(key)?,
            AppState::Load => (),
            AppState::Menu => (),
        };
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        match self.app_state {
            AppState::ManageUser => {
                let form = self.user_form.as_mut().unwrap();
                if form
                    .mouse_area()
                    .contains(Position::new(mouse.column, mouse.row))
                {
                    form.handle_mouse(mouse)?;
                }
            }
            _ => (),
        };

        Ok(())
    }
}
