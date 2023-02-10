use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "condition")]
pub enum Subscription {
    #[serde(rename = "channel.channel_points_custom_reward_redemption.add")]
    ChannelPointRedeem {
        broadcaster_user_id: String,
        reward_id: Option<String>,
    },
}
