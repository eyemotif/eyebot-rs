#[derive(Debug)]
pub enum ChatClientError {
    Irc(irc::error::Error),
    Access(crate::auth::access::AccessTokenManagerError),
    AuthIncomplete,
    AuthError(String),
    AuthUnrecognized(irc::proto::Message),
}

impl std::fmt::Display for ChatClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatClientError::Irc(error) => f.write_fmt(format_args!(
                "Chat error while using the IRC client: {error}"
            )),
            ChatClientError::Access(error) => f.write_fmt(format_args!(
                "Chat error while trying to get an Access Token: {error}",
            )),
            ChatClientError::AuthIncomplete => f.write_str(
                "Chat Auth error: Twitch closed the connection before all info could be received.",
            ),
            ChatClientError::AuthError(error) => {
                f.write_fmt(format_args!("Chat Auth error: {error}."))
            }
            ChatClientError::AuthUnrecognized(message) => f.write_fmt(format_args!(
                "Chat Auth: Unknown message {}.",
                message.to_string().trim()
            )),
        }
    }
}
impl std::error::Error for ChatClientError {}
impl From<irc::error::Error> for ChatClientError {
    fn from(value: irc::error::Error) -> Self {
        ChatClientError::Irc(value)
    }
}
impl From<crate::auth::access::AccessTokenManagerError> for ChatClientError {
    fn from(value: crate::auth::access::AccessTokenManagerError) -> Self {
        ChatClientError::Access(value)
    }
}
