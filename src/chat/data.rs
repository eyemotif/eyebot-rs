use crate::auth::access::AccessTokenManager;

#[derive(Debug)]
pub struct ChatClientData {
    pub access: AccessTokenManager,
    pub bot_username: String,
    pub chat_channel: String,
}

#[derive(Debug, Clone, Default)]
pub struct ChatMessage {
    pub id: String,
    pub channel: String,
    pub text: String,
    pub user_id: String,
    pub is_broadcaster: bool,
    pub is_moderator: bool,
    pub is_subscriber: bool,
}

impl ChatMessage {
    pub fn user_is_super(&self) -> bool {
        self.is_broadcaster || self.is_moderator
    }
}

impl PartialEq for ChatMessage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ChatMessage {}
