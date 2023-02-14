use crate::auth::access::AccessTokenManager;
use serde::Deserialize;

pub type WelcomeMessage = Message<payload::Welcome>;
pub type KeepaliveMessage = Message<payload::Keepalive>;
pub type NotificationMessage<E> = Message<payload::Notification<E>>;
pub type ReconnectMessage = Message<payload::Reconnect>;
pub type RevocationMessage = Message<payload::Revocation>;

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
    pub struct Notification<E> {
        pub subscription: Subscription,
        pub event: E,
    }
    #[derive(Debug, Deserialize)]
    pub struct Reconnect {
        pub session: EventSession,
    }
    #[derive(Debug, Deserialize)]
    pub struct Revocation {
        pub subscription: Subscription,
    }

    #[derive(Debug, Deserialize)]
    pub struct Subscription {
        pub id: String,
        pub status: String,
        pub version: String,
        #[serde(flatten)]
        pub subscription: super::super::subscription::Subscription,
    }
}

impl<P> PartialEq for Message<P> {
    fn eq(&self, other: &Self) -> bool {
        self.metadata.message_id == other.metadata.message_id
    }
}
impl<P> Eq for Message<P> {}
