#[derive(Debug)]
pub enum BotError {
    Chat(crate::chat::error::ChatClientError),
    Eventsub(crate::eventsub::error::EventsubError),
}

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotError::Chat(err) => f.write_fmt(format_args!("{err}")),
            BotError::Eventsub(err) => f.write_fmt(format_args!("{err}")),
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
