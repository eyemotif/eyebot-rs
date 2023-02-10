use serde::Deserialize;

use crate::auth::access::AccessTokenManager;

pub type WelcomeMessage = Message<payload::Welcome>;
pub type KeepaliveMessage = Message<payload::Keepalive>;
pub type NotificationMessage<E, C> = Message<payload::Notification<E, C>>;
pub type ReconnectMessage = Message<payload::Reconnect>;
pub type RevocationMessage<C> = Message<payload::Revocation<C>>;

#[derive(Debug)]
pub struct EventsubClientData {
    pub client_id: String,
    pub access: AccessTokenManager,
    pub subscriptions: Vec<super::subscription::Subscription>,
}

#[derive(Debug, Deserialize)]
pub struct Message<P> {
    pub metadata: MessageMetadata,
    pub payload: P,
}

#[derive(Debug, Deserialize)]
pub struct MessageMetadata {
    pub message_id: String,
    pub message_type: String,
    pub message_timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct EventSession {
    pub id: String,
    pub status: String,
    pub connected_at: String,
    pub keepalive_timeout_seconds: u64,
    pub reconnect_url: Option<String>,
}

pub mod payload {
    use super::EventSession;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Welcome {
        pub session: EventSession,
    }
    #[derive(Debug, Deserialize)]
    pub struct Keepalive {}
    #[derive(Debug, Deserialize)]
    pub struct Notification<E, C> {
        pub subscription: Subscription<C>,
        pub event: E,
    }
    #[derive(Debug, Deserialize)]
    pub struct Reconnect {
        pub session: EventSession,
    }
    #[derive(Debug, Deserialize)]
    pub struct Revocation<C> {
        pub subscription: Subscription<C>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Subscription<C> {
        pub id: String,
        pub status: String,
        #[serde(rename = "type")]
        pub event_type: String,
        pub version: String,
        pub condition: C,
    }
}
