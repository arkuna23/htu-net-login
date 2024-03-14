pub mod load_page;

use ratatui::{layout::Rect, Frame};
use tokio::sync::mpsc::UnboundedSender;

use crate::{data::{Action, Signal}, Result};

#[allow(unused_variables)]
pub trait Component {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }
    
    fn register_action_sender(&mut self, sender: UnboundedSender<Action>) -> Result<()> {
        Ok(())
    }

    fn handle_signal(&mut self, signal: Signal) -> Result<()> {
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, rect: Rect) -> Result<()> {
        Ok(())
    }
}
