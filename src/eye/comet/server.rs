use super::message::Message;
use super::CometInterface;
use ring::rand::SecureRandom;
use std::sync::{Arc, Weak};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::sync::{mpsc, watch};
use tokio_stream::StreamExt;
use tokio_tungstenite::tungstenite::Message as SocketMessage;

#[derive(Debug)]
pub struct Server {
    server: tokio::net::TcpListener,
    error_reporter: mpsc::Sender<crate::bot::error::BotError>,
    client: Option<Arc<Mutex<Client>>>,
    message_receiver: Arc<Mutex<mpsc::Receiver<super::message::TaggedMessage>>>,
    response_sender: Arc<watch::Sender<super::message::Response>>,
    interface: CometInterface,
}

#[derive(Debug)]
struct Client {
    connection: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    address: std::net::SocketAddr,
    state: String,
}

impl Server {
    pub async fn new(
        port: u16,
        error_reporter: tokio::sync::mpsc::Sender<crate::bot::error::BotError>,
    ) -> std::io::Result<Self> {
        let server = tokio::net::TcpListener::bind(format!("localhost:{port}")).await?;

        // TODO: remove magic number
        let (message_sender, message_receiver) = mpsc::channel(16);
        let (response_sender, response_receiver) = watch::channel(super::message::Response {
            state: String::new(),
            tag: super::message::MessageTag(Arc::new(String::new())),
            data: super::message::ResponseData::Ok,
        });

        Ok(Self {
            server,
            error_reporter,
            client: None,
            interface: CometInterface::new(message_sender, response_receiver),
            message_receiver: Arc::new(Mutex::new(message_receiver)),
            response_sender: Arc::new(response_sender),
        })
    }

    pub async fn accept_connections(mut self) {
        let rng = ring::rand::SystemRandom::new();
        // https://docs.rs/ring/latest/ring/rand/struct.SystemRandom.html
        match rng.fill(&mut []) {
            Ok(()) => (),
            Err(_) => {
                let _ = self
                    .error_reporter
                    .send(crate::bot::error::BotError::Custom(String::from(
                        "Could not create random data",
                    )))
                    .await;
                return;
            }
        }
        loop {
            let (connection, address) = match self.server.accept().await {
                Ok(it) => it,
                Err(err) => {
                    let _ = self
                        .error_reporter
                        .send(crate::bot::error::BotError::IO(err))
                        .await;
                    break;
                }
            };

            let mut connection = match tokio_tungstenite::accept_async(connection).await {
                Ok(it) => it,
                Err(err) => {
                    let _ = self
                        .error_reporter
                        .send(crate::bot::error::BotError::Custom(format!(
                            "Error on accepting a comet websocket connection: {err}"
                        )))
                        .await;
                    break;
                }
            };

            let state = match Server::create_state(&rng) {
                Ok(it) => it,
                Err(_) => {
                    let _ = self
                        .error_reporter
                        .send(crate::bot::error::BotError::Custom(String::from(
                            "Could not create random data",
                        )))
                        .await;
                    break;
                }
            };

            match connection
                .get_mut()
                .write_all(
                    &SocketMessage::Text(
                        serde_json::to_string(&Message::Register {
                            state: state.clone(),
                        })
                        .expect("Constant data"),
                    )
                    .into_data(),
                )
                .await
            {
                Ok(()) => (),
                Err(err) => {
                    let _ = self
                        .error_reporter
                        .send(crate::bot::error::BotError::Custom(format!(
                            "Error on sending a Register message to a comet websocket connection: {err}"
                        )))
                        .await;
                    break;
                }
            }

            self.interface.set_state(state.clone()).await;

            let client = Arc::new(Mutex::new(Client {
                connection,
                address,
                state,
            }));

            tokio::spawn(Server::handle_client(
                Arc::downgrade(&client),
                self.error_reporter.clone(),
                self.message_receiver.clone(),
                self.response_sender.clone(),
            ));

            match self.client.replace(client) {
                Some(old_client) => {
                    let _ = old_client.lock().await.connection.close(Some(
                    tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                        reason: "Server received new connection".into()
                    },
                    )).await;
                }
                None => (),
            }
        }
    }

    async fn handle_client(
        client: Weak<Mutex<Client>>,
        error_reporter: tokio::sync::mpsc::Sender<crate::bot::error::BotError>,
        message_receiver: Arc<Mutex<mpsc::Receiver<super::message::TaggedMessage>>>,
        response_sender: Arc<watch::Sender<super::message::Response>>,
    ) {
        tokio::join!(
            Server::client_inbound(client.clone(), error_reporter.clone(), response_sender),
            Server::client_outbound(client.clone(), error_reporter.clone(), message_receiver)
        );
    }

    async fn client_outbound(
        client: Weak<Mutex<Client>>,
        error_reporter: tokio::sync::mpsc::Sender<crate::bot::error::BotError>,
        message_receiver: Arc<Mutex<mpsc::Receiver<super::message::TaggedMessage>>>,
    ) {
        while let Some(message) = message_receiver.lock().await.recv().await {
            let Some(client) = client.upgrade() else { break; };

            let write_result = client.lock().await.connection.get_mut().write_all(&SocketMessage::Text(
                serde_json::to_string(&message).expect("Constant serialize")
            ).into_data()).await;

            match write_result {
                Ok(()) => (),
                Err(err) => {
                    let _ = 
                        error_reporter
                        .send(crate::bot::error::BotError::Custom(format!(
                            "Error on sending a Register message to a comet websocket connection: {err}"
                        )))
                        .await;
                }
            }
        }
    }

    async fn client_inbound(
        client: Weak<Mutex<Client>>,
        error_reporter: tokio::sync::mpsc::Sender<crate::bot::error::BotError>,
        response_sender: Arc<watch::Sender<super::message::Response>>,
    ) {
        loop {
            let Some(client) = client.upgrade() else { break; };
            let mut client = client.lock().await;
            match client.connection.next().await {
                Some(Ok(msg)) => match msg {
                    SocketMessage::Text(txt) => {
                        match serde_json::from_str::<super::message::Response>(&txt) {
                            Ok(response) => {
                                if response.state != client.state {
                                    let _ = client.connection.close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Protocol,
                                        reason: "Invalid state".into()
                                    })).await;
                                    break;
                                }

                                let _ = response_sender.send(response);
                            }
                            Err(err) => {
                                let _ = client.connection.close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Protocol,
                                        reason: format!("Malformed response: {err}").into()
                                    })).await;
                                break;
                            }
                        }
                    }
                    SocketMessage::Ping(data) => match client
                        .connection
                        .get_mut()
                        .write_all(&SocketMessage::Pong(data).into_data())
                        .await
                    {
                        Ok(()) => (),
                        Err(err) => {
                            let _ = error_reporter
                                .send(crate::bot::error::BotError::Custom(format!(
                                    "Error on sending a pong message to a comet websocket connection: {err}"
                                )))
                                .await;
                            break;
                        }
                    },
                    SocketMessage::Close(_) => break,
                    _ => (),
                },
                Some(Err(err)) => {
                    match err {
                        tokio_tungstenite::tungstenite::Error::ConnectionClosed
                        | tokio_tungstenite::tungstenite::Error::AlreadyClosed => (),
                        err => {
                            let _ = error_reporter
                                .send(crate::bot::error::BotError::Custom(format!(
                                    "Error on receiving from a comet websocket connection: {err}"
                                )))
                                .await;
                        }
                    }
                    break;
                }
                None => break,
            }
        }
    }

    fn create_state(rng: &ring::rand::SystemRandom) -> Result<String, ring::error::Unspecified> {
        let mut state = [0; 32];
        rng.fill(&mut state)?;
        Ok(state.into_iter().map(|byte| format!("{byte:x?}")).collect())
    }
}
