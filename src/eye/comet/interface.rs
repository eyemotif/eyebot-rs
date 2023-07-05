use super::feature::Feature;
use super::message::{Message, MessageTag, TaggedMessage};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, RwLock};

#[derive(Debug, Clone)]
pub struct CometInterface(Arc<RwLock<InterfaceData>>);

#[derive(Debug)]
struct InterfaceData {
    pub state: Arc<RwLock<Option<String>>>,
    features: Option<HashSet<Feature>>,
    message_sender: Option<mpsc::Sender<TaggedMessage>>,
    response_receiver: Option<watch::Receiver<super::message::Response>>,
}

impl CometInterface {
    pub(super) fn new(
        message_sender: mpsc::Sender<TaggedMessage>,
        response_receiver: watch::Receiver<super::message::Response>,
    ) -> Self {
        CometInterface(Arc::new(RwLock::new(InterfaceData {
            message_sender: Some(message_sender),
            response_receiver: Some(response_receiver),
            state: Arc::new(RwLock::new(None)),
            features: None,
        })))
    }

    pub fn send_message(
        &self,
        message: Message,
    ) -> impl std::future::Future<Output = Option<super::message::ResponseData>> {
        let data = self.0.clone();

        async move {
            let (mut response_receiver, tag, state) =
                Self::send_tagged_message(data, message).await?;

            while response_receiver.changed().await.is_ok() {
                if response_receiver.borrow().state != state {
                    continue;
                }

                if response_receiver.borrow().tag.is_close() {
                    break;
                }

                if response_receiver.borrow().tag.0 == tag.0 {
                    let response = response_receiver.borrow_and_update().data.clone();
                    return Some(response);
                }
            }

            None
        }
    }

    async fn send_tagged_message(
        data: Arc<RwLock<InterfaceData>>,
        message: Message,
    ) -> Option<(
        watch::Receiver<super::message::Response>,
        MessageTag,
        String,
    )> {
        let data = data.read().await;
        let tag = MessageTag::new();
        let state = data.state.read().await.as_ref()?.clone();

        let Ok(()) = data.message_sender
            .as_ref()?
            .send(TaggedMessage {
                tag: tag.clone(),
                state: state.clone(),
                message,
            })
            .await else { return None; };

        let response_receiver = data.response_receiver.as_ref()?.clone();

        Some((response_receiver, tag, state))
    }

    #[must_use]
    pub async fn has_client(&self) -> bool {
        self.0.read().await.state.read().await.is_some()
    }

    pub(super) async fn set_state(&self, new_state: String) {
        *self.0.read().await.state.write().await = Some(new_state);
    }
    pub(super) async fn set_disconnected(&self) {
        let mut interface = self.0.write().await;
        interface.message_sender.take();
        interface.response_receiver.take();
        *interface.state.write().await = None;
    }

    pub(super) async fn set_features(&self, features: HashSet<Feature>) {
        self.0.write().await.features = Some(features)
    }
    pub async fn get_features(&self) -> Option<HashSet<Feature>> {
        self.0.read().await.features.clone()
    }
}
