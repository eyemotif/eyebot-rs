#[derive(Debug)]
pub enum ChatClientError {
    Irc(irc::error::Error),
    Access(crate::auth::access::AccessTokenManagerError),
}

impl std::fmt::Display for ChatClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
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
