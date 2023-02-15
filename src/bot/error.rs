#[derive(Debug)]
pub enum BotError {
    Chat(crate::chat::error::ChatClientError),
    Eventsub(crate::eventsub::error::EventsubError),
    Say(irc::error::Error),
    Close,
    Custom(String),
}

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotError::Chat(err) => f.write_fmt(format_args!("{err}")),
            BotError::Eventsub(err) => f.write_fmt(format_args!("{err}")),
            BotError::Say(err) => f.write_fmt(format_args!(
                "Bot error while trying to post a message: {err}"
            )),
            BotError::Close => f.write_str("Bot is closing"),
            BotError::Custom(err) => f.write_fmt(format_args!("User-defined Bot error: {err}")),
        }
    }
}
impl std::error::Error for BotError {}

impl From<crate::chat::error::ChatClientError> for BotError {
    fn from(value: crate::chat::error::ChatClientError) -> Self {
        BotError::Chat(value)
    }
}
impl From<crate::eventsub::error::EventsubError> for BotError {
    fn from(value: crate::eventsub::error::EventsubError) -> Self {
        BotError::Eventsub(value)
    }
}
