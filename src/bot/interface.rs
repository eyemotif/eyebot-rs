use std::sync::Arc;

use super::error::BotError;
use crate::chat::interface::ChatInterface;
use crate::twitch::HelixAuth;

#[derive(Debug, Clone)]
pub struct BotInterface(pub(super) Arc<InterfaceData>);

#[derive(Debug)]
pub struct InterfaceData {
    pub helix_auth: HelixAuth,
    pub chat: ChatInterface,
    pub error_reporter: tokio::sync::mpsc::Sender<super::error::BotError>,
}

impl BotInterface {
    pub fn say<S: Into<String>>(&self, message: S) {
        if let Err(err) = self.0.chat.say(message) {
            // TODO: stop sending on error
            let _ = self.0.error_reporter.send(BotError::Say(err));
        }
    }
    pub fn reply<S: Into<String>>(&self, target: &crate::chat::data::ChatMessage, message: S) {
        if let Err(err) = self.0.chat.reply(target, message) {
            // TODO: stop sending on error
            let _ = self.0.error_reporter.send(BotError::Say(err));
        }
    }
    pub fn shutdown(self) {
        // TODO: stop sending on error
        let _ = self.0.error_reporter.send(BotError::Close);
    }
    pub fn error<S: Into<String>>(&self, error: S) {
        // TODO: stop sending on error
        let _ = self.0.error_reporter.send(BotError::Custom(error.into()));
    }
    pub fn helix_auth(&self) -> &HelixAuth {
        &self.0.helix_auth
    }
}
