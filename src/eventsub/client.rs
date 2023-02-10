use super::data::EventsubClientData;
use super::error::EventsubError;
use super::{data, outbound};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use tokio_tungstenite::tungstenite::Message;

type Websocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug)]
pub struct EventsubClient {
    websocket: Websocket,
    session_id: String,
    data: EventsubClientData,
}

impl EventsubClient {
    #[must_use]
    pub async fn new(data: EventsubClientData) -> Result<Self, EventsubError> {
        let websocket = EventsubClient::connect_websocket()
            .await
            .map_err(EventsubError::OnConnect)?;

        Ok(Self {
            session_id: String::new(),
            websocket,
            data,
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
        self.websocket
            .close(None)
            .await
            .map_err(EventsubError::OnReconnect)?;
        self.websocket = EventsubClient::connect_websocket()
            .await
            .map_err(EventsubError::OnReconnect)?;

        self.handle_welcome_message().await?;
        Ok(())
    }

    pub async fn run(mut self) -> Result<(), EventsubError> {
        self.handle_welcome_message().await?;

        Ok(())
    }

    async fn handle_welcome_message(&mut self) -> Result<(), EventsubError> {
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

                    outbound::send_subscriptions(
                        &self.data.subscriptions,
                        &self
                            .data
                            .access
                            .get_credentials()
                            .await
                            .map_err(EventsubError::Access)?
                            .access_token,
                    )
                    .await
                    .map_err(EventsubError::OnOutbound)?;

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

        return Err(EventsubError::WelcomeIncomplete);
    }
}
