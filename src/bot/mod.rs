use std::future::Future;

use crate::chat;
use crate::eventsub;
use crate::twitch;
use error::BotError;
use tokio::sync::mpsc;

pub mod data;
pub mod error;
pub mod interface;

#[derive(Debug)]
pub struct Bot {
    chat_client: chat::client::ChatClient,
    eventsub_client: eventsub::client::EventsubClient,
    interface: interface::BotInterface,
    error_listener: mpsc::Receiver<BotError>,
}

impl Bot {
    pub async fn new(data: data::BotData) -> Result<Self, BotError> {
        let chat_client = chat::client::ChatClient::new(chat::data::ChatClientData {
            access: data.access.clone(),
            bot_username: data.bot_username,
            chat_channel: data.chat_channel,
        })
        .await?;

        let eventsub_client =
            eventsub::client::EventsubClient::new(eventsub::data::EventsubClientData {
                client_id: data.client_id.clone(),
                access: data.access.clone(),
                subscriptions: data.subscriptions,
            })
            .await?;

        let helix_auth = twitch::HelixAuth {
            client_id: data.client_id,
            access: data.access,
        };

        let (error_sender, error_receiver) = mpsc::channel(1);

        Ok(Self {
            interface: interface::BotInterface(std::sync::Arc::new(interface::InterfaceData {
                helix_auth,
                chat: chat_client.get_interface(),
                error_reporter: error_sender,
            })),
            error_listener: error_receiver,
            chat_client,
            eventsub_client,
        })
    }

    pub fn on_chat_message<Fut: Future>(
        &self,
        mut f: impl FnMut(crate::chat::data::ChatMessage, interface::BotInterface) -> Fut,
    ) -> impl Future<Output = ()> {
        let interface = self.interface.clone();
        let mut receiver = self.chat_client.subscribe();

        async move {
            while receiver.changed().await.is_ok() {
                let chat_message = receiver.borrow().clone();
                f(chat_message, interface.clone()).await;
            }
        }
    }
    pub fn on_event<E: crate::eventsub::event::Event, Fut: Future>(
        &self,
        mut f: impl FnMut(crate::eventsub::data::NotificationMessage<E>, interface::BotInterface) -> Fut,
    ) -> impl Future<Output = ()> {
        let interface = self.interface.clone();
        let mut receiver = self.eventsub_client.subscribe();

        async move {
            while receiver.changed().await.is_ok() {
                let value = receiver.borrow().clone();
                if let Ok(value) = serde_json::from_value(value) {
                    f(value, interface.clone()).await;
                }
            }
        }
    }

    pub async fn run(mut self) -> Result<(), BotError> {
        tokio::select! {
            Err(chat_err) = self.chat_client.run() => Err(chat_err.into()),
            Err(eventsub_err) = self.eventsub_client.run() => Err(eventsub_err.into()),
            received_err = async {
                let Some(err) = self.error_listener.recv().await else {
                    loop { tokio::task::yield_now().await; }
                };
                self.error_listener.close();
                err
            } => Err(received_err),
            else => Ok(())
        }
    }
}
