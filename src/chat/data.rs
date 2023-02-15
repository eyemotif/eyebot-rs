use crate::auth::access::AccessTokenManager;
use std::collections::HashSet;

pub use super::tag::EmoteInfo;

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
    pub emotes: Vec<EmoteInfo>,
}

impl ChatMessage {
    #[must_use]
    pub fn user_is_super(&self) -> bool {
        self.is_broadcaster || self.is_moderator
    }
    #[must_use]
    pub fn strip_emotes(&self) -> String {
        let mut emote_locations = HashSet::new();
        for emote in &self.emotes {
            for (start, end) in &emote.locations {
                for index in *start..=*end {
                    emote_locations.insert(index);
                }
            }
        }
        self.text
            .char_indices()
            .filter_map(|(loc, chr)| (!emote_locations.contains(&(loc as u16))).then_some(chr))
            .collect()
    }
}

impl PartialEq for ChatMessage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ChatMessage {}
