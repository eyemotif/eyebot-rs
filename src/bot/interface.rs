use std::sync::Arc;

use crate::chat::interface::ChatInterface;
use crate::twitch::HelixAuth;

#[derive(Debug, Clone)]
pub struct BotInterface(pub(super) Arc<InterfaceData>);

#[derive(Debug)]
pub struct InterfaceData {
    pub helix_auth: HelixAuth,
    pub chat: ChatInterface,
}
