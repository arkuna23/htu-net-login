use std::sync::atomic::{AtomicU16, Ordering};

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use lazy_static::lazy_static;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::data::{Action, Signal};

use super::{Component, ComponentInfo};

#[derive(Debug)]
pub struct Input {
    mouse_area: Rect,
    content: String,
    id: u16,
    action_tx: Option<UnboundedSender<Action>>,
    name: String,
    selected: bool,
    password: bool,
}

lazy_static! {
    static ref MAX_ID: AtomicU16 = AtomicU16::new(0);
}

impl Default for Input {
    fn default() -> Self {
        Self {
            content: Default::default(),
            id: { MAX_ID.fetch_add(1, Ordering::SeqCst) },
            name: Default::default(),
            action_tx: None,
            selected: Default::default(),
            mouse_area: Default::default(),
            password: false,
        }
    }
}

impl Input {
    pub fn new(name: &str) -> Self {
        Self {
            content: Default::default(),
            id: { MAX_ID.fetch_add(1, Ordering::SeqCst) },
            action_tx: Default::default(),
            name: name.into(),
            selected: Default::default(),
            mouse_area: Default::default(),
            password: false,
        }
    }
}

impl Component for Input {
    fn init(&mut self) -> crate::Result<super::ComponentInfo> {
        Ok(ComponentInfo::all_enabled())
    }

    fn register_action_sender(&mut self, sender: UnboundedSender<Action>) -> crate::Result<()> {
        self.action_tx = Some(sender);
        Ok(())
    }

    fn draw(
        &mut self,
        f: &mut ratatui::prelude::Frame,
        rect: ratatui::prelude::Rect,
    ) -> crate::Result<()> {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.name().len() as u16),
                Constraint::Length(1),
                Constraint::Length(16),
            ])
            .split(rect);

        f.render_widget(self.name().white().bold(), layout[0]);

        let mut text = if self.password {
            "*".repeat(self.content().len())
        } else {
            self.content().into()
        };
        if text.len() < 16 {
            text += " ".repeat(16 - self.content().len()).as_str();
        } else if self.selected {
            text = text[self.content().len() - 16..].to_owned();
        } else {
            text = text[0..16].to_owned();
        }

        let span = if self.selected {
            text.white().on_dark_gray()
        } else {
            text.white().on_black()
        };

        f.render_widget(span, layout[2]);
        self.mouse_area = layout[2];

        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::Result<()> {
        if let KeyEventKind::Press | KeyEventKind::Repeat = key.kind {
            match key.code {
                KeyCode::Backspace => {
                    let s = self.content_mut();
                    if !s.is_empty() {
                        s.truncate(s.len() - 1);
                        self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
                    }
                }
                KeyCode::Char(c) => {
                    self.content_mut().push(c);
                    self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
                }
                _ => (),
            }
        };

        Ok(())
    }

    fn handle_mouse(&mut self, _mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        if let MouseEventKind::Down(MouseButton::Left) = _mouse.kind {
            self.action_tx
                .as_ref()
                .unwrap()
                .send(Action::SelectInput(self.id))
                .unwrap();
        }
        Ok(())
    }

    fn handle_signal(&mut self, signal: crate::data::Signal) -> crate::Result<()> {
        if let Signal::InputSelected(id) = signal {
            self.selected = id == self.id();
        }

        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }
}

impl Input {
    pub fn content(&self) -> &str {
        self.content.as_str()
    }

    pub fn toggle_password(&mut self) {
        self.password = !self.password;
    }

    pub fn content_mut(&mut self) -> &mut String {
        &mut self.content
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn toggle_select(&mut self) {
        self.selected = !self.selected;
    }

    pub fn length(&self) -> u16 {
        self.name.len() as u16 + 1 + 16
    }
}
