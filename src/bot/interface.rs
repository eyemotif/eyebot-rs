use super::error::BotError;
use crate::chat::interface::ChatInterface;
use crate::twitch::HelixAuth;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct BotInterface(pub(super) Arc<InterfaceData>);

#[derive(Debug)]
pub struct InterfaceData {
    pub(super) helix_auth: HelixAuth,
    pub(super) chat: ChatInterface,
    pub(super) error_reporter: tokio::sync::mpsc::Sender<super::error::BotError>,
    pub(super) message_history: Arc<(Mutex<VecDeque<String>>, usize)>,
}

impl BotInterface {
    pub async fn say<S: Into<String>>(&self, message: S) {
        let message = message.into();

        let (history, cap) = &*self.0.message_history;
        let mut history = history.lock().await;
        if history.contains(&message) {
            return;
        }

        if let Err(err) = self.0.chat.say(message.clone()) {
            drop(history);
            let _ = self.0.error_reporter.send(BotError::Say(err)).await;
            return;
        }

        if *cap > 0 {
            if history.len() >= *cap {
                history.pop_front();
            }
            history.push_back(message);
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
    #[must_use]
    pub fn helix_auth(&self) -> &HelixAuth {
        &self.0.helix_auth
    }
    pub fn mock_message<S: Into<String>>(&self, mock: &crate::chat::data::ChatMessage, text: S) {
        self.0.chat.mock_message(mock.clone(), text);
    }
}
