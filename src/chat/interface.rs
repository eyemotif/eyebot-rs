use super::data::ChatMessage;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct ChatInterface(pub(super) Arc<InterfaceData>);

#[derive(Debug)]
pub(super) struct InterfaceData {
    pub(super) twitch_channel: String,
    pub(super) irc_client: Arc<irc::client::Client>,
    pub(super) message_channel: watch::Sender<ChatMessage>,
}

impl ChatInterface {
    pub(super) fn new(irc_client: Arc<irc::client::Client>, twitch_channel: String) -> Self {
        Self(Arc::new(InterfaceData {
            twitch_channel,
            irc_client,
            message_channel: watch::channel(ChatMessage::default()).0,
        }))
    }

    pub fn say<S: Into<String>>(&self, message: S) -> irc::error::Result<()> {
        self.0.irc_client.send(irc::proto::Command::PRIVMSG(
            format!("#{}", self.0.twitch_channel),
            message.into(),
        ))
    }
    pub fn reply<S: Into<String>>(
        &self,
        target: &ChatMessage,
        message: S,
    ) -> irc::error::Result<()> {
        self.0.irc_client.send(irc::proto::Message {
            tags: Some(vec![irc::proto::message::Tag(
                String::from("reply-parent-msg-id"),
                Some(target.id.clone()),
            )]),
            prefix: None,
            command: irc::proto::Command::PRIVMSG(
                format!("#{}", self.0.twitch_channel),
                message.into(),
            ),
        })
    }
    pub fn mock_message<S: Into<String>>(&self, mock: ChatMessage, text: S) {
        let _ = self.0.message_channel.send(ChatMessage {
            text: text.into(),
            ..mock
        });
    }
}
