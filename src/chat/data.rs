use crate::auth::access::AccessTokenManager;

#[derive(Debug)]
pub struct ChatConnectionData {
    pub access: AccessTokenManager,
    pub bot_username: String,
    pub chat_channel: String,
}
