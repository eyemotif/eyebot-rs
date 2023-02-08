use crate::auth::access::AccessTokenManager;
use irc::client::Client;
use std::sync::Arc;

#[derive(Debug)]
pub struct ChatClientData {
    pub access: AccessTokenManager,
    pub bot_username: String,
    pub chat_channel: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub(super) client: Arc<Client>,
    pub channel: String,
    pub message: String,
}

impl ChatMessage {
    pub fn say<S: Into<String>>(&self, message: S) {
        self.client
            .send(irc::proto::Command::PRIVMSG(
                format!("#{}", self.channel),
                message.into(),
            ))
            .expect("TODO: handle ChatMessage.say() error")
    }
}
