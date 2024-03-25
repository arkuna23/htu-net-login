pub mod checkbox;
pub mod input;

use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::Block,
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::data::{Action, AppPage, Signal, UserInfo};

use self::{checkbox::Group, input::Input};

use super::{
    util::{centered_box_sized, horizontal_centered},
    Component, ComponentInfo,
};

enum SelectedInput {
    Id,
    Pwd,
}

impl Default for SelectedInput {
    fn default() -> Self {
        Self::Id
    }
}

#[derive(Default)]
pub struct AccountForm {
    mouse_area: Rect,
    btn_mouse_area: Rect,
    pong: bool,
    id: Input,
    password: Input,
    selected: SelectedInput,
    checkbox_group: Group,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Component for AccountForm {
    fn init(&mut self) -> crate::Result<super::ComponentInfo> {
        self.id.name_mut().push_str("学号");
        self.password.name_mut().push_str("密码");
        self.id.init()?;
        self.password.init()?;
        self.checkbox_group.init()?;
        self.id.toggle_select();
        self.password.toggle_password();
        Ok(ComponentInfo::all_enabled())
    }

    fn register_action_sender(
        &mut self,
        sender: tokio::sync::mpsc::UnboundedSender<crate::data::Action>,
    ) -> crate::Result<()> {
        self.id.register_action_sender(sender.clone())?;
        self.password.register_action_sender(sender.clone())?;
        self.checkbox_group.register_action_sender(sender.clone())?;
        self.action_tx = Some(sender);
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::Result<()> {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Enter => {
                    self.action_tx
                        .as_ref()
                        .unwrap()
                        .send(Action::SetAccount(UserInfo {
                            id: self.id.content().into(),
                            password: self.password.content().into(),
                            suffix: self.checkbox_group.selected(),
                        }))
                        .unwrap();
                    return Ok(());
                }
                KeyCode::Tab => {
                    match self.selected {
                        SelectedInput::Id => self.selected = SelectedInput::Pwd,
                        SelectedInput::Pwd => self.selected = SelectedInput::Id,
                    }
                    self.id.toggle_select();
                    self.password.toggle_select();
                    self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
                }
                KeyCode::Esc => {
                    self.action_tx
                        .as_ref()
                        .unwrap()
                        .send(Action::JumpTo(AppPage::Menu))
                        .unwrap();
                    return Ok(());
                }
                _ => (),
            };
        }

        match self.selected {
            SelectedInput::Id => self.id.handle_key(key),
            SelectedInput::Pwd => self.password.handle_key(key),
        }
    }

    fn handle_signal(&mut self, signal: crate::data::Signal) -> crate::Result<()> {
        match signal {
            Signal::DaemonPong => {
                if !self.pong {
                    self.action_tx
                        .as_ref()
                        .unwrap()
                        .send(Action::GetAccount)
                        .unwrap();
                    self.pong = true;
                }
            }
            Signal::UserInfo(user) => {
                *self.id.content_mut() = user.id;
                *self.password.content_mut() = user.password;
                self.checkbox_group.select(user.suffix);
                self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
            }
            Signal::InputSelected(id) => {
                self.id.handle_signal(Signal::InputSelected(id))?;
                self.password.handle_signal(signal)?;
                if id == self.id.id() {
                    self.selected = SelectedInput::Id;
                } else if id == self.password.id() {
                    self.selected = SelectedInput::Pwd;
                }
                self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
            }
            Signal::CheckboxSelected(_) => self.checkbox_group.handle_signal(signal)?,
            Signal::Exit => (),
            _ => (),
        };

        Ok(())
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        let pos = Position::new(mouse.column, mouse.row);
        if self.id.mouse_area().contains(pos) {
            self.id.handle_mouse(mouse)
        } else if self.password.mouse_area().contains(pos) {
            self.password.handle_mouse(mouse)
        } else if self.checkbox_group.mouse_area().contains(pos) {
            self.checkbox_group.handle_mouse(mouse)
        } else if self.btn_mouse_area.contains(pos) {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                self.action_tx
                    .as_ref()
                    .unwrap()
                    .send(Action::SetAccount(UserInfo {
                        id: self.id.content().into(),
                        password: self.password.content().into(),
                        suffix: self.checkbox_group.selected(),
                    }))
                    .unwrap();
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn draw(&mut self, f: &mut Frame, rect: ratatui::prelude::Rect) -> crate::Result<()> {
        let block = Block::bordered()
            .border_style(Style::default().blue())
            .title("账号设置")
            .title_bottom(Line::from("<ESC>返回上一级菜单").centered());
        let centered = centered_box_sized(rect, 32, 12);
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(block.inner(centered.centered));
        f.render_widget(block, centered.centered);

        self.id
            .draw(f, horizontal_centered(self.id.length(), layout[0]))?;
        self.password
            .draw(f, horizontal_centered(self.password.length(), layout[2]))?;
        self.checkbox_group.draw(f, layout[4])?;

        let btn = Line::from(vec![
            "设定".red().bold().on_light_blue(),
            "<Enter>".light_red().on_light_blue(),
        ]);
        let btn_layout = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(btn.width() as u16),
            Constraint::Min(0),
        ])
        .split(layout[6]);
        f.render_widget(btn, btn_layout[1]);
        self.btn_mouse_area = btn_layout[1];
        self.mouse_area = rect;

        Ok(())
    }

    fn tick(&mut self) -> crate::Result<()> {
        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }
}
