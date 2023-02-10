use super::error::EventsubError;

type Websocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug)]
pub struct EventsubClient {
    websocket: Websocket,
}

impl EventsubClient {
    pub async fn new() -> Result<Self, EventsubError> {
        let (websocket, _) = tokio_tungstenite::connect_async_tls_with_config(
            "wss://eventsub-beta.wss.twitch.tv/ws",
            None,
            Some(tokio_tungstenite::Connector::Rustls(
                super::tls::create_websocket_tls_client(),
            )),
        )
        .await
        .map_err(EventsubError::Connect)?;

        Ok(Self { websocket })
    }
}
