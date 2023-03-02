use super::feature::Feature;
use super::message::{Message, MessageTag, TaggedMessage};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex, RwLock};

#[derive(Debug, Clone)]
pub struct CometInterface(Arc<Mutex<InterfaceData>>);

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
        CometInterface(Arc::new(Mutex::new(InterfaceData {
            message_sender: Some(message_sender),
            response_receiver: Some(response_receiver),
            state: Arc::new(RwLock::new(None)),
            features: None,
        })))
    }

    #[must_use]
    pub fn send_message(
        &self,
        message: Message,
    ) -> impl std::future::Future<Output = Option<super::message::ResponseData>> {
        let data = self.0.clone();

        async move {
            let data = data.lock().await;

            let tag = MessageTag::new();

            let state = data
                .state
                .read()
                .await
                .as_ref()
                .expect("Interface should have its state set")
                .clone();

            data.message_sender
                .as_ref()
                .expect("Comet server should be open")
                .send(TaggedMessage {
                    tag: tag.clone(),
                    state: state.clone(),
                    message,
                })
                .await
                .expect("Comet server should be open");

            let mut response_receiver = data
                .response_receiver
                .as_ref()
                .map(|r| r.clone())
                .expect("Comet server should be open");

            drop(data);

            while response_receiver.changed().await.is_ok() {
                if response_receiver.borrow().state == "CLOSE" {
                    break;
                }
                if response_receiver.borrow().state != state {
                    continue;
                }

                if response_receiver.borrow().tag.0 == tag.0 {
                    let response = response_receiver.borrow_and_update().data.clone();
                    return Some(response);
                }
            }

            None
        }
    }

    #[must_use]
    pub async fn has_client(&self) -> bool {
        self.0.lock().await.state.read().await.is_some()
    }

    pub(super) async fn set_state(&self, new_state: String) {
        *self.0.lock().await.state.write().await = Some(new_state);
    }
    pub(super) async fn set_disconnected(&self) {
        let mut interface = self.0.lock().await;
        interface.message_sender.take();
        interface.response_receiver.take().map(|receiver| receiver);
        *interface.state.write().await = None;
        *self.0.lock().await.state.write().await = None;
    }

    pub(super) async fn get_features_mut(
        &self,
    ) -> tokio::sync::MappedMutexGuard<'_, Option<HashSet<Feature>>> {
        tokio::sync::MutexGuard::map(self.0.lock().await, |inner| &mut inner.features)
    }
    pub async fn get_features(&self) -> Option<HashSet<Feature>> {
        self.0.lock().await.features.clone()
    }
}
