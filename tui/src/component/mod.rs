pub mod container;
pub mod form;
pub mod menu;
pub mod page;
pub mod util;

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{layout::Rect, Frame};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    data::{Action, Signal},
    Result,
};

#[derive(Default, Clone)]
pub struct ComponentInfo {
    pub mouse_enabled: bool,
    pub key_enabled: bool,
}

#[allow(unused_variables)]
pub trait Component {
    fn init(&mut self) -> Result<ComponentInfo> {
        Ok(ComponentInfo::default())
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

    fn tick(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        Ok(())
    }

    fn mouse_area(&self) -> Rect {
        todo!()
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        Ok(())
    }
}

impl ComponentInfo {
    pub fn all_enabled() -> Self {
        Self {
            mouse_enabled: true,
            key_enabled: true,
        }
    }
}

pub struct EmptyComponent;

impl Component for EmptyComponent {}

impl Default for Box<dyn Component> {
    fn default() -> Self {
        Box::new(EmptyComponent)
    }
}
