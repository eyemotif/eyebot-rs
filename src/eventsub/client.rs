use super::data::{self, EventsubClientData, NotificationMessage};
use super::error::EventsubError;
use super::event::Event;
use super::outbound;
use futures_util::StreamExt;
use std::future::Future;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

type Websocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug)]
pub struct EventsubClient {
    websocket: Websocket,
    session_id: String,
    data: EventsubClientData,
    interface: watch::Sender<serde_json::Value>,
    options: crate::options::Options,
}

impl EventsubClient {
    #[must_use]
    pub async fn new(
        data: EventsubClientData,
        options: crate::options::Options,
    ) -> Result<Self, EventsubError> {
        options.debug("Eventsub: Connecting to Twitch");

        let websocket = EventsubClient::connect_websocket()
            .await
            .map_err(EventsubError::OnConnect)?;

        Ok(Self {
            session_id: String::new(),
            websocket,
            data,
            interface: watch::channel(serde_json::Value::Null).0,
            options,
        })
    }

    async fn connect_websocket() -> tokio_tungstenite::tungstenite::Result<Websocket> {
        let (websocket, _) = tokio_tungstenite::connect_async_tls_with_config(
            "wss://eventsub-beta.wss.twitch.tv/ws",
            None,
            Some(tokio_tungstenite::Connector::Rustls(
                super::tls::create_websocket_tls_client(),
            )),
        )
        .await?;
        Ok(websocket)
    }

    async fn reconnect(&mut self) -> Result<(), EventsubError> {
        self.options.debug("Eventsub: Reconnecting to twitch");

        self.websocket
            .close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                reason: std::borrow::Cow::Owned(String::from("Reconnecting.")),
            }))
            .await
            .map_err(EventsubError::OnReconnect)?;
        self.websocket = EventsubClient::connect_websocket()
            .await
            .map_err(EventsubError::OnReconnect)?;

        self.handle_welcome_message().await?;
        Ok(())
    }

    pub fn on_message<T: serde::de::DeserializeOwned, Fut: Future>(
        &self,
        mut f: impl FnMut(T) -> Fut,
    ) -> impl Future<Output = ()> {
        let mut receiver = self.interface.subscribe();
        async move {
            while receiver.changed().await.is_ok() {
                let value = receiver.borrow().clone();
                if let Ok(value) = serde_json::from_value(value) {
                    f(value).await;
                }
            }
        }
    }
    pub fn on_event<E: Event, Fut: Future>(
        &self,
        f: impl FnMut(NotificationMessage<E>) -> Fut,
    ) -> impl Future<Output = ()> {
        self.on_message(f)
    }

    pub async fn run(mut self) -> Result<(), EventsubError> {
        self.handle_welcome_message().await?;
        self.handle_messages().await?;

        Ok(())
    }

    pub fn subscribe(&self) -> watch::Receiver<serde_json::Value> {
        self.interface.subscribe()
    }

    async fn handle_messages(mut self) -> Result<(), EventsubError> {
        self.options.debug("Eventsub: Ready to receive messages!");

        while let Some(message) = self
            .websocket
            .next()
            .await
            .transpose()
            .map_err(EventsubError::OnReceive)?
        {
            match message {
                Message::Text(text) => {
                    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
                        return Err(EventsubError::ReceiveInvalid);
                    };

                    if let Ok(message) =
                        serde_json::from_value::<data::ReconnectMessage>(json.clone())
                    {
                        if message.metadata.message_type == "session_reconnect" {
                            self.reconnect().await?;
                            continue;
                        }
                    }
                    if let Ok(message) =
                        serde_json::from_value::<data::KeepaliveMessage>(json.clone())
                    {
                        if message.metadata.message_type == "session_keepalive" {
                            continue;
                        }
                    }

                    // TODO: stop sending on error
                    let _ = self.interface.send(json);
                }
                Message::Ping(data) => self
                    .websocket
                    .get_mut()
                    .write_all(&Message::Pong(data).into_data())
                    .await
                    .map_err(EventsubError::OnPong)?,
                Message::Close(_) => break,
                _ => (),
            }
        }
        Ok(())
    }
    async fn handle_welcome_message(&mut self) -> Result<(), EventsubError> {
        self.options.debug("Eventsub: Receiving Welcome message");

        while let Some(message) = self
            .websocket
            .next()
            .await
            .transpose()
            .map_err(EventsubError::OnWelcome)?
        {
            match message {
                Message::Text(text) => {
                    let Ok(welcome) = serde_json::from_str::<data::WelcomeMessage>(&text) else {
                        return Err(EventsubError::WelcomeInvalid)
                    };
                    self.session_id = welcome.payload.session.id;

                    self.options.debug(format!(
                        "Eventsub: Subscribing to events (session id: {})",
                        self.session_id
                    ));

                    outbound::send_subscriptions(
                        &self.data.subscriptions,
                        &self.session_id,
                        &crate::twitch::HelixAuth {
                            client_id: self.data.client_id.clone(),
                            access: self.data.access.clone(),
                        },
                    )
                    .await?;

                    return Ok(());
                }
                Message::Ping(data) => self
                    .websocket
                    .get_mut()
                    .write_all(&Message::Pong(data).into_data())
                    .await
                    .map_err(EventsubError::OnPong)?,
                _ => break,
            }
        }

        Err(EventsubError::WelcomeIncomplete)
    }
}
