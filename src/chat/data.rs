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
    pub(super) id: String,

    pub channel: String,
    pub text: String,
    pub user_id: String,
    pub is_broadcaster: bool,
    pub is_moderator: bool,
    pub is_subscriber: bool,
}

impl ChatMessage {
    pub fn is_super(&self) -> bool {
        self.is_broadcaster || self.is_moderator
    }

    pub fn say<S: Into<String>>(&self, message: S) {
        self.client
            .send(irc::proto::Command::PRIVMSG(
                format!("#{}", self.channel),
                message.into(),
            ))
            .expect("TODO: handle ChatMessage.say() error")
    }
    pub fn reply<S: Into<String>>(&self, message: S) {
        self.client
            .send(irc::proto::Message {
                tags: Some(vec![irc::proto::message::Tag(
                    String::from("reply-parent-msg-id"),
                    Some(self.id.clone()),
                )]),
                prefix: None,
                command: irc::proto::Command::PRIVMSG(format!("#{}", self.channel), message.into()),
            })
            .expect("TODO: handle ChatMessage.reply() error")
    }
    pub(super) fn empty(client: Arc<Client>) -> Self {
        Self {
            client,
            id: Default::default(),
            channel: Default::default(),
            text: Default::default(),
            is_broadcaster: Default::default(),
            is_moderator: Default::default(),
            is_subscriber: Default::default(),
            user_id: Default::default(),
        }
    }
}

impl PartialEq for ChatMessage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ChatMessage {}
