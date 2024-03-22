use std::sync::atomic::AtomicU16;

use crossterm::event::{MouseButton, MouseEventKind};
use lazy_static::lazy_static;
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::Stylize,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    component::{Component, ComponentInfo},
    data::{Action, Signal, Suffix},
};

pub struct Checkbox {
    id: u16,
    mouse_area: Rect,
    label: String,
    checked: bool,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Default for Checkbox {
    fn default() -> Self {
        let id = CHECKBOX_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            mouse_area: Rect::default(),
            label: String::new(),
            checked: false,
            action_tx: None,
        }
    }
}

lazy_static! {
    static ref CHECKBOX_ID: AtomicU16 = 0.into();
}

impl Component for Checkbox {
    fn init(&mut self) -> crate::Result<crate::component::ComponentInfo> {
        Ok(ComponentInfo {
            mouse_enabled: true,
            key_enabled: false,
        })
    }

    fn register_action_sender(&mut self, sender: UnboundedSender<Action>) -> crate::Result<()> {
        self.action_tx = Some(sender);
        Ok(())
    }

    fn handle_signal(&mut self, signal: crate::data::Signal) -> crate::Result<()> {
        if let Signal::CheckboxSelected(id) = signal {
            self.checked = self.id == id;
        }

        Ok(())
    }

    fn draw(&mut self, f: &mut ratatui::prelude::Frame, rect: Rect) -> crate::Result<()> {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((self.label.len() - 2) as u16),
                Constraint::Length(2),
            ])
            .split(rect);

        if self.checked {
            f.render_widget(self.label().light_green(), layout[0]);
            f.render_widget("✅".on_green(), layout[1]);
        } else {
            f.render_widget(self.label().white(), layout[0]);
            f.render_widget("  ".on_black(), layout[1]);
        }
        self.mouse_area = layout[1];

        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            self.action_tx
                .as_ref()
                .unwrap()
                .send(Action::SelectCheckbox(self.id))
                .unwrap();
        }
        Ok(())
    }
}

impl Checkbox {
    pub fn checked(&self) -> bool {
        self.checked
    }

    pub fn checked_mut(&mut self) -> &mut bool {
        &mut self.checked
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn label_mut(&mut self) -> &mut String {
        &mut self.label
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn length(&self) -> u16 {
        (self.label.chars().count() + 2) as u16
    }
}

#[derive(Default)]
pub struct Group {
    china_mobile: Checkbox,
    china_unicom: Checkbox,
    china_telecom: Checkbox,
    local: Checkbox,
    selected: Suffix,
    mouse_area: Rect,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Group {
    fn update_struct(&mut self) {
        *self.china_mobile.checked_mut() = false;
        *self.china_telecom.checked_mut() = false;
        *self.china_unicom.checked_mut() = false;
        *self.local.checked_mut() = false;
        match self.selected {
            Suffix::ChinaMobile => *self.china_mobile.checked_mut() = true,
            Suffix::ChinaUnicom => *self.china_unicom.checked_mut() = true,
            Suffix::ChinaTelecom => *self.china_telecom.checked_mut() = true,
            Suffix::Local => *self.local.checked_mut() = true,
        };
    }

    pub fn select(&mut self, suffix: Suffix) {
        self.selected = suffix;
        self.update_struct();
    }

    pub fn length(&self) -> u16 {
        self.china_mobile.length()
            + self.china_telecom.length()
            + self.china_unicom.length()
            + self.local.length()
    }

    pub fn selected(&self) -> Suffix {
        self.selected
    }
}

impl Component for Group {
    fn init(&mut self) -> crate::Result<ComponentInfo> {
        self.china_mobile.label_mut().push_str("移动");
        self.china_unicom.label_mut().push_str("联通");
        self.china_telecom.label_mut().push_str("电信");
        self.local.label_mut().push_str("其他");
        self.update_struct();
        Ok(ComponentInfo {
            mouse_enabled: true,
            key_enabled: false,
        })
    }

    fn register_action_sender(&mut self, sender: UnboundedSender<Action>) -> crate::Result<()> {
        self.china_mobile.register_action_sender(sender.clone())?;
        self.china_unicom.register_action_sender(sender.clone())?;
        self.china_telecom.register_action_sender(sender.clone())?;
        self.local.register_action_sender(sender.clone())?;
        self.action_tx = Some(sender);
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        let pos = Position::new(mouse.column, mouse.row);
        if self.china_mobile.mouse_area.contains(pos) {
            self.china_mobile.handle_mouse(mouse)
        } else if self.china_telecom.mouse_area.contains(pos) {
            self.china_telecom.handle_mouse(mouse)
        } else if self.china_unicom.mouse_area.contains(pos) {
            self.china_unicom.handle_mouse(mouse)
        } else if self.local.mouse_area.contains(pos) {
            self.local.handle_mouse(mouse)
        } else {
            Ok(())
        }
    }

    fn draw(&mut self, f: &mut ratatui::prelude::Frame, rect: Rect) -> crate::Result<()> {
        let layout = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Min(0),
            Constraint::Min(0),
            Constraint::Min(0),
        ])
        .split(rect);
        self.mouse_area = rect;

        self.china_mobile.draw(f, layout[0])?;
        self.china_unicom.draw(f, layout[1])?;
        self.china_telecom.draw(f, layout[2])?;
        self.local.draw(f, layout[3])?;

        Ok(())
    }

    fn handle_signal(&mut self, signal: Signal) -> crate::Result<()> {
        if let Signal::CheckboxSelected(id) = signal {
            if id == self.china_mobile.id() {
                self.selected = Suffix::ChinaMobile;
            } else if id == self.china_unicom.id() {
                self.selected = Suffix::ChinaUnicom;
            } else if id == self.china_telecom.id() {
                self.selected = Suffix::ChinaTelecom;
            } else if id == self.local.id() {
                self.selected = Suffix::Local;
            }
            self.china_mobile
                .handle_signal(Signal::CheckboxSelected(id))?;
            self.china_unicom
                .handle_signal(Signal::CheckboxSelected(id))?;
            self.china_telecom
                .handle_signal(Signal::CheckboxSelected(id))?;
            self.local.handle_signal(signal)?;
            self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
        }

        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }
}
