use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::data::{Action, Signal};

use super::{
    input::Input,
    util::{centered_box_sized, mouse_contains},
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
pub struct UserForm {
    mouse_area: Rect,
    id: Option<Input>,
    password: Option<Input>,
    selected: SelectedInput,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Component for UserForm {
    fn init(&mut self) -> crate::Result<super::ComponentInfo> {
        let mut id = Input::new("学号");
        let mut password = Input::new("密码");
        id.init()?;
        password.init()?;
        id.toggle_select();
        self.id = Some(id);
        self.password = Some(password);
        Ok(ComponentInfo::all_enabled())
    }

    fn register_action_sender(
        &mut self,
        sender: tokio::sync::mpsc::UnboundedSender<crate::data::Action>,
    ) -> crate::Result<()> {
        self.id
            .as_mut()
            .unwrap()
            .register_action_sender(sender.clone())?;
        self.password
            .as_mut()
            .unwrap()
            .register_action_sender(sender.clone())?;
        sender.send(Action::GetUser).unwrap();
        self.action_tx = Some(sender);
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::Result<()> {
        match self.selected {
            SelectedInput::Id => self.id.as_mut().unwrap().handle_key(key),
            SelectedInput::Pwd => self.password.as_mut().unwrap().handle_key(key),
        }
    }

    fn handle_signal(&mut self, signal: crate::data::Signal) -> crate::Result<()> {
        match signal {
            Signal::UserInfo(user) => {
                *self.id.as_mut().unwrap().content_mut() = user.id;
                *self.password.as_mut().unwrap().content_mut() = user.password;
                self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
            }
            Signal::InputSelected(id) => {
                self.id
                    .as_mut()
                    .unwrap()
                    .handle_signal(Signal::InputSelected(id))?;
                self.password.as_mut().unwrap().handle_signal(signal)?;
                if id == self.id.as_ref().unwrap().id() {
                    self.selected = SelectedInput::Id;
                } else if id == self.password.as_ref().unwrap().id() {
                    self.selected = SelectedInput::Pwd;
                }
            }
            Signal::Exit => (),
            _ => (),
        };

        Ok(())
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        let (contains, id) = mouse_contains(&mouse, self.id.as_mut());
        if contains {
            id.handle_mouse(mouse)?;
            return Ok(());
        }
        let (contains, pwd) = mouse_contains(&mouse, self.password.as_mut());
        if contains {
            pwd.handle_mouse(mouse)?;
        }

        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, rect: ratatui::prelude::Rect) -> crate::Result<()> {
        let centered = centered_box_sized(rect, 32, 16);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Min(0), Constraint::Min(0)])
            .split(centered);

        self.id.as_mut().unwrap().draw(f, layout[0])?;
        self.password.as_mut().unwrap().draw(f, layout[1])?;
        self.mouse_area = rect;

        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }
}
