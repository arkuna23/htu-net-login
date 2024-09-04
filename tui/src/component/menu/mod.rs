use crossterm::event::{KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::Block,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::data::{Action, AppPage};

use super::{util::centered_box_sized, Component, ComponentInfo};

#[derive(Clone, Copy)]
enum Selection {
    SetUser,
    Logout,
}

impl Default for Selection {
    fn default() -> Self {
        Self::SetUser
    }
}

#[derive(Default)]
pub struct Menu {
    selecton: usize,
    mouse_area: Rect,
    menu: [Selection; 2],
    sel_mouse_area: [Rect; 2],
    action_tx: Option<UnboundedSender<Action>>,
}

impl Component for Menu {
    fn init(&mut self) -> crate::Result<super::ComponentInfo> {
        self.menu = [Selection::SetUser, Selection::Logout];
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
        let areas = centered_box_sized(rect, 32, 12);
        let block = Block::bordered()
            .border_style(Style::default().blue())
            .title("菜单")
            .title_bottom(Line::from("键盘上下切换选项").centered());
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(block.inner(areas.centered));
        f.render_widget(block, areas.centered);
        f.render_widget(
            "鼠标左键/<Enter>确定选项".yellow().to_centered_line(),
            layout[3],
        );
        f.render_widget("设定账号".reset(), layout[0]);
        f.render_widget("登出校园网".reset(), layout[1]);
        match self.menu[self.selecton] {
            Selection::SetUser => f.render_widget("> 设定账号".green().underlined(), layout[0]),
            Selection::Logout => f.render_widget("> 登出校园网".red().underlined(), layout[1]),
        }

        self.sel_mouse_area = [layout[0], layout[1]];
        self.mouse_area = areas.centered;
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            let pos = Position::new(mouse.column, mouse.row);
            for i in 0..self.sel_mouse_area.len() {
                if self.sel_mouse_area[i].contains(pos) {
                    self.selecton = i;
                    self.execute(self.menu[i]);
                    self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::Result<()> {
        if key.kind != KeyEventKind::Release {
            match key.code {
                KeyCode::Up => {
                    self.selecton = (self.selecton + self.menu.len() - 1) % self.menu.len();
                    self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
                }
                KeyCode::Down => {
                    self.selecton = (self.selecton + 1) % self.menu.len();
                    self.action_tx.as_ref().unwrap().send(Action::Draw).unwrap();
                }
                KeyCode::Enter => self.execute(self.menu[self.selecton]),
                _ => (),
            };
        }

        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }
}

impl Menu {
    fn execute(&self, selection: Selection) {
        match selection {
            Selection::SetUser => self
                .action_tx
                .as_ref()
                .unwrap()
                .send(Action::JumpTo(AppPage::Form))
                .unwrap(),
            Selection::Logout => self
                .action_tx
                .as_ref()
                .unwrap()
                .send(Action::Logout)
                .unwrap(),
        }
    }
}
