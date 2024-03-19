use ratatui::{
    layout::{Position, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::data::Action;

use super::{page::Page, Component, ComponentInfo};

#[derive(Default)]
pub struct AppContainer {
    mouse_area: Rect,
    page: Option<Page>,
    page_info: Option<ComponentInfo>,
    act_tx: Option<UnboundedSender<Action>>,
}

impl Component for AppContainer {
    fn init(&mut self) -> crate::Result<ComponentInfo> {
        self.page = Some(Page::default());
        self.page_info = Some(self.page.as_mut().unwrap().init()?);
        Ok(ComponentInfo {
            mouse_enabled: true,
            key_enabled: true,
        })
    }

    fn register_action_sender(
        &mut self,
        sender: tokio::sync::mpsc::UnboundedSender<crate::data::Action>,
    ) -> crate::Result<()> {
        self.act_tx = Some(sender.clone());
        sender.send(Action::Draw).unwrap();
        self.page
            .as_mut()
            .unwrap()
            .register_action_sender(sender.clone())?;
        Ok(())
    }

    fn handle_signal(&mut self, signal: crate::data::Signal) -> crate::Result<()> {
        self.page.as_mut().unwrap().handle_signal(signal)
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> crate::Result<()> {
        if self.page_info.as_ref().unwrap().mouse_enabled {
            let page = self.page.as_mut().unwrap();
            if page
                .mouse_area()
                .contains(Position::new(mouse.column, mouse.row))
            {
                page.handle_mouse(mouse)?;
            }
        }
        Ok(())
    }

    fn draw(
        &mut self,
        f: &mut ratatui::prelude::Frame,
        rect: ratatui::prelude::Rect,
    ) -> crate::Result<()> {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Line::from("HTU NET LOGIN").centered())
            .border_style(Style::default().red());
        let inner_area = block.inner(rect);
        f.render_widget(block, rect);
        self.page.as_mut().unwrap().draw(f, inner_area)?;
        self.mouse_area = rect;
        Ok(())
    }

    fn tick(&mut self) -> crate::Result<()> {
        self.page.as_mut().unwrap().tick()?;
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> crate::Result<()> {
        if self.page_info.as_ref().unwrap().key_enabled {
            self.page.as_mut().unwrap().handle_key(key)
        } else {
            Ok(())
        }
    }

    fn mouse_area(&self) -> Rect {
        self.mouse_area
    }
}
