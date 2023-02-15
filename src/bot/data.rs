#[derive(Debug)]
pub struct BotData {
    pub client_id: String,
    pub access: crate::auth::access::AccessTokenManager,
    pub bot_username: String,
    pub chat_channel: String,
    pub subscriptions: Vec<crate::eventsub::subscription::Subscription>,
}
