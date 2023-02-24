use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;

/// Trait is sealed.
pub trait Event: DeserializeOwned + sealed::Sealed {}
mod sealed {
    use super::{ChannelPointRedeem, Raid, StreamOnline, Subscription};
    pub trait Sealed {}
    impl Sealed for ChannelPointRedeem {}
    impl Sealed for Subscription {}
    impl Sealed for Raid {}
    impl Sealed for StreamOnline {}
}

#[derive(Debug, Deserialize)]
pub struct ChannelPointRedeem {
    pub id: String,
    pub user_name: String,
    pub user_id: String,
    pub user_input: Option<String>,
    pub reward: Reward,
}
#[derive(Debug, Deserialize)]
pub struct Subscription {
    pub user_name: String,
    pub user_id: String,
    pub tier: String,
    pub cumulative_months: u16,
    pub streak_months: Option<u16>,
    pub duration_months: u16,
    pub message: SubscriptionMessage,
}
#[derive(Debug, Deserialize)]
pub struct Raid {
    pub from_broadcaster_user_id: String,
    pub from_broadcaster_user_name: String,
    pub to_broadcaster_user_id: String,
    pub to_broadcaster_user_name: String,
    pub viewers: u32,
}
#[derive(Debug, Deserialize)]
pub struct StreamOnline {
    #[serde(rename = "type")]
    pub stream_type: String,
    pub started_at: String,
}

impl Event for ChannelPointRedeem {}
impl Event for Subscription {}
impl Event for Raid {}
impl Event for StreamOnline {}

#[derive(Debug, Deserialize)]
pub struct Reward {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct SubscriptionMessage {
    pub text: String,
    pub emotes: Vec<SubscriptionEmote>,
}
#[derive(Debug, Deserialize)]
pub struct SubscriptionEmote {
    pub begin: u16,
    pub end: u16,
    pub id: String,
}

impl SubscriptionMessage {
    #[must_use]
    pub fn get_emote_info(&self) -> Vec<crate::chat::data::EmoteInfo> {
        let mut map = HashMap::<String, Vec<(u16, u16)>>::new();
        for emote in &self.emotes {
            if let Some(emote_locs) = map.get_mut(&emote.id) {
                emote_locs.push((emote.begin, emote.end));
            } else {
                map.insert(emote.id.clone(), vec![(emote.begin, emote.end)]);
            }
        }
        map.into_iter()
            .map(|(id, locations)| crate::chat::data::EmoteInfo { id, locations })
            .collect()
    }
}

impl PartialEq for Reward {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Reward {}
