use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Paragraph},
};
use tokio::{
    sync::mpsc::UnboundedSender,
    time::{self, Instant},
};

use crate::data::{Action, AppPage, DaemonRequest, Level, Notification, Signal};

use super::{form::AccountForm, menu::Menu, util::str_to_lines, Component, ComponentInfo};

#[derive(Default)]
pub struct Page {
    action_tx: Option<UnboundedSender<Action>>,
    notification: Option<Notification>,
    inner_info: ComponentInfo,
    inner: Box<dyn Component>,
    last_pong: Option<Instant>,
    mouse_area: Rect,
    exit_state: Arc<AtomicBool>,
    connected: bool,
}

impl Component for Page {
    fn init(&mut self) -> crate::Result<super::ComponentInfo> {
        let mut menu = Menu::default();
        self.inner_info = menu.init()?;
        self.inner = Box::new(menu);
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

        if let Some(notification) = self.notification.as_ref() {
            let noti_lay =
                Layout::horizontal([Constraint::Min(0), Constraint::Min(0), Constraint::Min(0)])
                    .split(layout[0])[2];
            let block = Block::bordered().border_type(BorderType::Rounded);
            let lines = {
                let mut v = vec![Line::from({
                    match notification.level {
                        Level::Info => "信息".bold().green(),
                        Level::Error => "错误".bold().red(),
                    }
                })
                .left_aligned()];
                str_to_lines(
                    notification.msg.as_str(),
                    noti_lay.width - 2,
                    Style::default(),
                    &mut v,
                );
                v
            };

            let noti_layout =
                Layout::vertical([Constraint::Length(lines.len() as u16 + 2)]).split(noti_lay);
            f.render_widget(
                Paragraph::new(Text::from(lines)).block(block),
                noti_layout[0],
            );
        }
        Ok(())
    }

    fn handle_signal(&mut self, signal: crate::data::Signal) -> crate::Result<()> {
        match &signal {
            Signal::DaemonPong => {
                self.last_pong = Some(Instant::now());
            }
            Signal::Exit => {
                self.exit_state
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            Signal::ChangePage(page) => {
                let mut com: Box<dyn Component> = match page {
                    AppPage::Menu => Box::<Menu>::default(),
                    AppPage::Form => Box::<AccountForm>::default(),
                };
                com.init()?;
                com.register_action_sender(self.action_tx.as_ref().unwrap().clone())
                    .unwrap();
                self.inner = com;
                self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
            }
            Signal::DaemonResponse { req, result } => match req {
                DaemonRequest::SetAccount => match result {
                    Ok(_) => self.popup_notification(Level::Info, "账号信息设置成功".into()),
                    Err(e) => self.popup_notification(Level::Error, format!("{:?}", e)),
                },
                DaemonRequest::Logout => match result {
                    Ok(_) => self.popup_notification(Level::Info, "登出成功".into()),
                    Err(e) => self.popup_notification(Level::Error, format!("{:?}", e)),
                },
            },
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

        if let Some(n) = self.notification.as_ref() {
            if Instant::now().duration_since(n.time).as_secs() > 3 {
                self.notification = None;
                self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
            }
        }
        self.inner.tick()?;

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

impl Page {
    pub fn popup_notification(&mut self, level: Level, msg: String) {
        self.notification = Some(Notification::new(level, msg));
        self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
    }
}
