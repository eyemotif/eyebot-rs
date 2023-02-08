use crate::auth::access::AccessTokenManager;

#[derive(Debug)]
pub struct ChatClientData {
    pub access: AccessTokenManager,
    pub bot_username: String,
    pub chat_channel: String,
}
