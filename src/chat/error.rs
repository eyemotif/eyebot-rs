#[derive(Debug)]
pub enum ChatConnectionError {
    Irc(irc::error::Error),
    Access(crate::auth::access::AccessTokenManagerError),
}

impl std::fmt::Display for ChatConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl std::error::Error for ChatConnectionError {}
impl From<irc::error::Error> for ChatConnectionError {
    fn from(value: irc::error::Error) -> Self {
        ChatConnectionError::Irc(value)
    }
}
impl From<crate::auth::access::AccessTokenManagerError> for ChatConnectionError {
    fn from(value: crate::auth::access::AccessTokenManagerError) -> Self {
        ChatConnectionError::Access(value)
    }
}
