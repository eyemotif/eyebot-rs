use super::message::{Message, MessageTag, TaggedMessage};
use ring::rand::SecureRandom;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex, RwLock};

#[derive(Debug)]
pub struct CometInterface(Arc<Mutex<InterfaceData>>);

#[derive(Debug)]
struct InterfaceData {
    pub state: Arc<RwLock<Option<String>>>,
    message_sender: Option<mpsc::Sender<TaggedMessage>>,
    response_receiver: Option<watch::Receiver<super::message::Response>>,
    rng: ring::rand::SystemRandom,
}

impl CometInterface {
    pub(super) fn new(
        message_sender: mpsc::Sender<TaggedMessage>,
        response_receiver: watch::Receiver<super::message::Response>,
    ) -> Self {
        let rng = ring::rand::SystemRandom::new();
        // https://docs.rs/ring/latest/ring/rand/struct.SystemRandom.html
        let _ = rng.fill(&mut []);

        CometInterface(Arc::new(Mutex::new(InterfaceData {
            message_sender: Some(message_sender),
            response_receiver: Some(response_receiver),
            rng: ring::rand::SystemRandom::new(),
            state: Arc::new(RwLock::new(None)),
        })))
    }

    pub(super) async fn set_state(&self, new_state: String) {
        *self.0.lock().await.state.write().await = Some(new_state);
    }

    async fn create_message_id(&self) -> String {
        let data = self.0.lock().await;
        loop {
            let mut state = [0; 32];
            match data.rng.fill(&mut state) {
                Ok(()) => return state.into_iter().map(|byte| format!("{byte:x?}")).collect(),
                Err(_) => (),
            }
        }
    }

    pub fn send_message<Fut: std::future::Future>(
        &self,
        message: Message,
        on_response: impl FnOnce(super::message::Response) -> Fut,
    ) -> impl std::future::Future<Output = ()> {
        let data = self.0.clone();

        async move {
            let mut data = data.lock().await;

            let tag = loop {
                let mut state = [0; 32];
                match data.rng.fill(&mut state) {
                    Ok(()) => break state.into_iter().map(|byte| format!("{byte:x?}")).collect(),
                    Err(_) => (),
                };
            };
            let tag = MessageTag(Arc::new(tag));

            let state = data
                .state
                .read()
                .await
                .as_ref()
                .expect("Interface should have its state set")
                .clone();

            match data
                .message_sender
                .as_ref()
                .expect("Comet server should be open")
                .send(TaggedMessage {
                    tag: tag.clone(),
                    state,
                    message,
                })
                .await
            {
                Ok(()) => (),
                Err(_) => {
                    data.message_sender = None;
                    return;
                }
            }

            let Some(mut response_receiver) = data.response_receiver.as_ref().map(|r| r.clone()) else {return; };
            drop(data);

            while response_receiver.changed().await.is_ok() {
                if response_receiver.borrow().tag.0 == tag.0 {
                    let response = response_receiver.borrow_and_update().clone();
                    on_response(response).await;
                    break;
                }
            }
        }
    }
}
