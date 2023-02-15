use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "condition")]
pub enum Subscription {
    #[serde(rename = "channel.channel_points_custom_reward_redemption.add")]
    ChannelPointRedeem {
        broadcaster_user_id: String,
        reward_id: Option<String>,
    },
    #[serde(rename = "channel.subscription.message")]
    Subscription { broadcaster_user_id: String },
    #[serde(rename = "channel.raid")]
    RaidTo {
        #[serde(rename = "to_broadcaster_user_id")]
        broadcaster_user_id: String,
    },
    #[serde(rename = "channel.raid")]
    RaidFrom {
        #[serde(rename = "from_broadcaster_user_id")]
        broadcaster_user_id: String,
    },
    #[serde(rename = "stream.online")]
    StreamOnline { broadcaster_user_id: String },
}
