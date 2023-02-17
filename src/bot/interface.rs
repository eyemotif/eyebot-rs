use std::sync::Arc;

use super::error::BotError;
use crate::chat::interface::ChatInterface;
use crate::twitch::HelixAuth;

#[derive(Debug)]
pub struct BotInterface(pub(super) Arc<InterfaceData>);

#[derive(Debug)]
pub struct InterfaceData {
    pub helix_auth: HelixAuth,
    pub chat: ChatInterface,
    pub error_reporter: tokio::sync::mpsc::Sender<super::error::BotError>,
}

impl BotInterface {
    pub async fn say<S: Into<String>>(&self, message: S) {
        if let Err(err) = self.0.chat.say(message) {
            let _ = self.0.error_reporter.send(BotError::Say(err)).await;
        }
    }
    pub async fn reply<S: Into<String>>(
        &self,
        target: &crate::chat::data::ChatMessage,
        message: S,
    ) {
        if let Err(err) = self.0.chat.reply(target, message) {
            let _ = self.0.error_reporter.send(BotError::Say(err)).await;
        }
    }
    pub async fn shutdown(self) {
        let _ = self.0.error_reporter.send(BotError::Close).await;
    }
    pub async fn error<S: Into<String>>(&self, error: S) {
        let _ = self
            .0
            .error_reporter
            .send(BotError::Custom(error.into()))
            .await;
    }
    #[must_use] pub fn helix_auth(&self) -> &HelixAuth {
        &self.0.helix_auth
    }
}
