use futures_util::{SinkExt, StreamExt};
use ring::rand::SecureRandom;
use std::sync::{Arc, Weak};
use tokio::sync::{broadcast, mpsc, watch, Mutex};
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::{Error as SocketError, Message as SocketMessage};

pub mod component;
pub mod feature;
mod interface;
mod message;

pub use interface::CometInterface;
pub use message::{Message, Response, ResponseData};

#[derive(Debug)]
pub struct Server {
    server: tokio::net::TcpListener,
    error_reporter: mpsc::Sender<crate::bot::error::BotError>,
    client: Option<Arc<Client>>,
    message_receiver: Arc<Mutex<mpsc::Receiver<message::TaggedMessage>>>,
    response_sender: Arc<watch::Sender<message::Response>>,
    interface: CometInterface,
    options: crate::options::Options,
    streamer_username: String,
}

#[derive(Debug)]
struct Client {
    sender: Mutex<
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
            SocketMessage,
        >,
    >,
    receiver: Mutex<
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        >,
    >,
    state: String,
    close_sender: broadcast::Sender<()>,
}

macro_rules! close_err {
    () => {
        Err(SocketError::ConnectionClosed)
            | Err(SocketError::AlreadyClosed)
            | Err(SocketError::Protocol(ProtocolError::SendAfterClosing))
            | Err(SocketError::Protocol(ProtocolError::ReceivedAfterClosing))
    };
}

async fn wait_for<T, Fut: std::future::Future<Output = T>>(
    mut close_receiver: broadcast::Receiver<()>,
    fut: Fut,
) -> Option<T> {
    tokio::select! {
        output = fut => Some(output),
        _ = close_receiver.recv() => None,
    }
}

impl Server {
    pub async fn new<S: Into<String>>(
        port: u16,
        streamer_username: S,
        error_reporter: mpsc::Sender<crate::bot::error::BotError>,
        options: crate::options::Options,
    ) -> std::io::Result<Self> {
        let server = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

        options.debug(format!(
            "Comet: Bound server to port {}",
            server.local_addr().expect("Address should be set").port()
        ));

        // TODO: remove magic number
        let (message_sender, message_receiver) = mpsc::channel(16);
        let (response_sender, response_receiver) = watch::channel(message::Response {
            state: String::new(),
            tag: message::MessageTag::close(),
            data: message::ResponseData::Ok,
        });

        Ok(Self {
            server,
            error_reporter,
            client: None,
            interface: CometInterface::new(message_sender, response_receiver),
            message_receiver: Arc::new(Mutex::new(message_receiver)),
            response_sender: Arc::new(response_sender),
            options,
            streamer_username: streamer_username.into(),
        })
    }

    pub async fn accept_connections(mut self) {
        self.options.debug("Comet: Accepting connections!");

        loop {
            let (connection, _) = match self.server.accept().await {
                Ok(it) => it,
                Err(err) => {
                    let _ = self
                        .error_reporter
                        .send(crate::bot::error::BotError::IO(err))
                        .await;
                    break;
                }
            };

            let socket = match tokio_tungstenite::accept_async(connection).await {
                Ok(it) => it,
                Err(SocketError::Protocol(_)) => continue,
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

            self.options.debug(format!(
                "Comet: New connection @ {}",
                socket
                    .get_ref()
                    .peer_addr()
                    .expect("Connected socket should have peer address")
            ));

            let (mut sender, receiver) = socket.split();

            let state = match Server::create_state() {
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

            // FIXME: on a new connection, this doesn't fire sometimes
            match sender
                .send(SocketMessage::Text(
                    serde_json::to_string(&self::message::TaggedMessage {
                        message: Message::Register {
                            state: state.clone(),
                        },
                        state: state.clone(),
                        tag: message::MessageTag::new(),
                    })
                    .expect("Constant data should serialize"),
                ))
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

            self.options
                .debug(format!("Comet: Registered client (state: {state})"));

            let (close_sender, _) = broadcast::channel(1);
            let client = Arc::new(Client {
                sender: Mutex::new(sender),
                receiver: Mutex::new(receiver),
                state: state.clone(),
                close_sender: close_sender.clone(),
            });

            tokio::spawn(Server::handle_client(
                Arc::downgrade(&client),
                client.short_state(),
                self.error_reporter.clone(),
                self.message_receiver.clone(),
                self.response_sender.clone(),
                close_sender,
                self.interface.clone(),
                self.options,
                self.streamer_username.clone(),
            ));

            match self.client.replace(client) {
                Some(old_client) => {
                    let _ = old_client.sender.lock().await.send(SocketMessage::Close(Some(
                    tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                        reason: "Server received new connection".into()
                    },
                    ))).await;

                    // Flush any threads waiting on a response
                    let _ = self.response_sender.send(Response {
                        state: old_client.state.clone(),
                        tag: message::MessageTag::close(),
                        data: ResponseData::Error {
                            is_internal: true,
                            message: String::from("This error should never be handled"),
                        },
                    });
                }
                None => (),
            }
        }
    }

    pub fn interface(&self) -> CometInterface {
        self.interface.clone()
    }

    async fn handle_client(
        client: Weak<Client>,
        task_name: String,
        error_reporter: mpsc::Sender<crate::bot::error::BotError>,
        message_receiver: Arc<Mutex<mpsc::Receiver<message::TaggedMessage>>>,
        response_sender: Arc<watch::Sender<message::Response>>,
        close_sender: broadcast::Sender<()>,
        interface: CometInterface,
        options: crate::options::Options,
        streamer_username: String,
    ) {
        tokio::join!(
            Server::client_ping(
                client.clone(),
                &task_name,
                error_reporter.clone(),
                close_sender.clone(),
                options
            ),
            Server::client_inbound(
                client.clone(),
                &task_name,
                error_reporter.clone(),
                close_sender.clone(),
                response_sender,
                options
            ),
            Server::client_outbound(
                client.clone(),
                &task_name,
                error_reporter.clone(),
                close_sender.clone(),
                message_receiver,
                options
            ),
            Server::client_features(
                &task_name,
                error_reporter.clone(),
                close_sender.clone(),
                options,
                interface.clone(),
                streamer_username,
            ),
        );

        //FIXME: deadlock occurs here
        interface.set_disconnected().await;
        options.debug(format!("Comet ({task_name}): Client disconnected!"))
    }

    async fn client_outbound(
        client: Weak<Client>,
        task_name: &str,
        error_reporter: mpsc::Sender<crate::bot::error::BotError>,
        close_sender: broadcast::Sender<()>,
        message_receiver: Arc<Mutex<mpsc::Receiver<message::TaggedMessage>>>,
        options: crate::options::Options,
    ) {
        options.debug(format!("Comet ({task_name}): Accepting outbound messages!"));

        loop {
            let Some(Some(message)) = wait_for(close_sender.subscribe(), async {message_receiver.lock().await.recv().await}).await else {break};

            let Some(client) = client.upgrade() else { break; };

            options.debug(format!(
                "Comet ({task_name}): Outbound: {:?}",
                message.message
            ));

            let write_result = client
                .sender
                .lock()
                .await
                .send(SocketMessage::Text(
                    serde_json::to_string(&message).expect("Data should serialize"),
                ))
                .await;

            match write_result {
                Ok(()) => (),
                close_err!() => break,
                Err(err) => {
                    let _ = error_reporter
                        .send(crate::bot::error::BotError::Custom(
                            // cargo fmt doesn't format huge strings
                            String::from(
                                "Error on sending a Register message to a comet
                            websocket connection:",
                            ) + &err.to_string(),
                        ))
                        .await;
                }
            }
        }

        options.debug(format!("Comet ({task_name}): Outbound task closed"));
    }

    async fn client_inbound(
        client: Weak<Client>,
        task_name: &str,
        error_reporter: mpsc::Sender<crate::bot::error::BotError>,
        close_sender: broadcast::Sender<()>,
        response_sender: Arc<watch::Sender<message::Response>>,
        options: crate::options::Options,
    ) {
        options.debug(format!("Comet ({task_name}): Accepting inbound messages!"));

        loop {
            let Some(client) = client.upgrade() else { break; };

            match wait_for(close_sender.subscribe(), async {
                client.receiver.lock().await.next().await
            })
            .await
            {
                Some(Some(Ok(msg))) => match msg {
                    SocketMessage::Text(txt) => {
                        match serde_json::from_str::<message::Response>(&txt) {
                            Ok(response) => {
                                if response.state != client.state {
                                    let _ = client.sender.lock().await.send(SocketMessage::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Protocol,
                                        reason: "Invalid state".into()
                                    }))).await;

                                    let _ = client.close_sender.send(());
                                    break;
                                }

                                options.debug(format!(
                                    "Comet ({task_name}): Inbound: {:?}",
                                    response.data
                                ));

                                let _ = response_sender.send(response);
                            }
                            Err(err) => {
                                let _ = client.sender.lock().await.send(SocketMessage::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Protocol,
                                        reason: format!("Malformed response: {err}").into()
                                    }))).await;

                                let _ = client.close_sender.send(());
                                break;
                            }
                        }
                    }
                    SocketMessage::Ping(data) => {
                        match client
                            .sender
                            .lock()
                            .await
                            .send(SocketMessage::Pong(data))
                            .await
                        {
                            Ok(()) => (),
                            close_err!() => break,
                            Err(err) => {
                                let _ = error_reporter
                                .send(crate::bot::error::BotError::Custom(format!(
                                    "Error on sending a pong message to a comet websocket connection: {err}"
                                )))
                                .await;
                                break;
                            }
                        }
                    }
                    SocketMessage::Close(_) => {
                        options.debug(format!("Comet ({task_name}): Client sent close message"));
                        let _ = client.close_sender.send(());
                        break;
                    }
                    _ => (),
                },
                Some(Some(Err(err))) => {
                    match err {
                        SocketError::ConnectionClosed
                        | SocketError::AlreadyClosed
                        | SocketError::Protocol(ProtocolError::SendAfterClosing)
                        | SocketError::Protocol(ProtocolError::ReceivedAfterClosing) => (),
                        _ => {
                            let _ = error_reporter
                                .send(crate::bot::error::BotError::Custom(format!(
                                    "Error on receiving from a comet websocket connection: {err}"
                                )))
                                .await;
                        }
                    }
                    let _ = client.close_sender.send(());
                    break;
                }
                _ => {
                    let _ = client.close_sender.send(());
                    break;
                }
            }; // semicolon is required for drop checker
        }
        options.debug(format!("Comet ({task_name}): Inbound task closed"));
    }

    async fn client_ping(
        client: Weak<Client>,
        task_name: &str,
        error_reporter: mpsc::Sender<crate::bot::error::BotError>,
        close_sender: broadcast::Sender<()>,
        options: crate::options::Options,
    ) {
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(10));

        options.debug(format!("Comet ({task_name}): Starting ping task"));
        loop {
            ping_interval.tick().await;
            let Some(client) = client.upgrade() else { break; };

            let ping_data: Vec<u8> = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Unix epoch should be earlier than now")
                .as_millis()
                .to_ne_bytes()
                .to_vec();

            let Some(ping_result) = wait_for(close_sender.subscribe(), async {
                client
                    .sender
                    .lock()
                    .await
                    .send(SocketMessage::Ping(ping_data))
                    .await
            }).await else { break };

            match ping_result {
                Ok(()) => (),
                close_err!() => {
                    let _ = client.close_sender.send(());
                    break;
                }
                Err(err) => {
                    let _ = error_reporter
                        .send(crate::bot::error::BotError::Custom(format!(
                            "Error on sending a Ping message to a Comet client: {err}"
                        )))
                        .await;

                    break;
                }
            }
        }

        options.debug(format!("Comet ({task_name}): Ping task closed"));
    }

    async fn client_features(
        task_name: &str,
        error_reporter: mpsc::Sender<crate::bot::error::BotError>,
        close_sender: broadcast::Sender<()>,
        options: crate::options::Options,
        interface: CometInterface,
        streamer_username: String,
    ) {
        options.debug(format!("Comet ({task_name}): Initializing features..."));

        let Some(Some(features)) = wait_for(close_sender.subscribe(),feature::Feature::get_features(interface.clone())).await else { return; };

        match feature::Feature::init(interface.clone(), features.clone(), streamer_username)
            .await
            .expect("Client should be connected")
        {
            Ok(()) => (),
            Err(err) => {
                let _ = error_reporter
                    .send(crate::bot::error::BotError::Custom(format!(
                        "Error on initializing comet features: {err}"
                    )))
                    .await;
                return;
            }
        }

        options.debug(format!(
            "Comet ({task_name}): Initialized features {features:?}"
        ))
    }

    fn create_state() -> Result<String, ring::error::Unspecified> {
        lazy_static::lazy_static!(
            static ref RNG: ring::rand::SystemRandom = ring::rand::SystemRandom::new();
        );

        let mut state = [0; 32];
        RNG.fill(&mut state)?;
        Ok(state.into_iter().map(|byte| format!("{byte:x?}")).collect())
    }
}

impl Client {
    pub fn short_state(&self) -> String {
        let mut id = self.state.clone();
        id.truncate(4);
        id + ".."
    }
}
