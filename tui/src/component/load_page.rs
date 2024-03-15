use tokio::sync::mpsc::UnboundedSender;

use crate::data::Action;

use super::Component;

#[derive(Default)]
pub struct LoadPage {
    action_tx: Option<UnboundedSender<Action>>,
}

impl Component for LoadPage {
    fn register_action_sender(&mut self, sender: UnboundedSender<Action>) -> crate::Result<()> {
        self.action_tx = Some(sender);
        Ok(())
    }
}

