use ratatui::{
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::data::{Action, AppState};

use super::{load_page::LoadPage, Component};

#[derive(Default)]
pub struct AppContainer {
    state: AppState,
    load_page: Option<LoadPage>,
    act_tx: Option<UnboundedSender<Action>>,
}

impl Component for AppContainer {
    fn init(&mut self) -> crate::Result<()> {
        self.load_page = Some(LoadPage::default());
        Ok(())
    }

    fn register_action_sender(
        &mut self,
        sender: tokio::sync::mpsc::UnboundedSender<crate::data::Action>,
    ) -> crate::Result<()> {
        self.act_tx = Some(sender.clone());
        sender.send(Action::Draw).unwrap();
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
            .title("HTU NET LOGIN")
            .border_style(Style::default().red());
        let inner_area = block.inner(rect);
        f.render_widget(block, rect);
        match self.state {
            AppState::Load => self.load_page.as_mut().unwrap().draw(f, inner_area)?,
            AppState::Menu => todo!(),
            AppState::ManageUser => todo!(),
        }
        Ok(())
    }
}
