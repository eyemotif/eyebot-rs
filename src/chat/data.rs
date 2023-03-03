pub use super::tag::EmoteInfo;
use crate::auth::access::AccessTokenManager;
use std::collections::HashSet;

#[derive(Debug)]
pub struct ChatClientData {
    pub access: ChatAccess,
    pub bot_username: String,
    pub chat_channel: String,
}

#[derive(Debug)]
pub enum ChatAccess {
    Authorization(AccessTokenManager),
    Implicit(String),
}

#[derive(Debug, Clone, Default)]
pub struct ChatMessage {
    pub id: String,
    pub channel: String,
    pub text: String,
    pub user_id: String,
    pub is_moderator: bool,
    pub is_subscriber: bool,
    pub emotes: Vec<EmoteInfo>,
    pub display_name: String,
    pub name_color: Option<String>,
    pub badges: std::collections::HashMap<String, String>,
}

impl ChatMessage {
    #[must_use]
    pub fn user_is_broadcaster(&self) -> bool {
        self.badges.contains_key("broadcaster")
    }
    #[must_use]
    pub fn user_is_super(&self) -> bool {
        self.is_moderator || self.user_is_broadcaster()
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

    pub async fn get_badges(
        &self,
        broadcaster_id: &str,
        auth: &crate::twitch::HelixAuth,
    ) -> Result<Vec<crate::twitch::TwitchBadgeUrls>, Box<dyn std::error::Error + Send + Sync>> {
        let badges = crate::twitch::get_all_badges(broadcaster_id, auth).await?;
        Ok(badges
            .into_iter()
            .filter_map(|(name, badges)| self.badges.get(&name).map(|version| (version, badges)))
            .filter_map(|(version, badges)| badges.into_iter().find(|urls| version == &urls.id))
            .collect())
    }
}

impl PartialEq for ChatMessage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ChatMessage {}
