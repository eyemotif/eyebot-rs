use super::data::ChatMessage;
use super::error::ChatClientError;
use irc::client::Client;
use irc::proto::{Command, Response};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;

#[derive(Debug)]
pub struct ChatClient {
    client: Arc<Client>,
    stream: irc::client::ClientStream,
    data: super::data::ChatClientData,
    sender: watch::Sender<ChatMessage>,
}

impl ChatClient {
    pub async fn new(data: super::data::ChatClientData) -> Result<Self, ChatClientError> {
        let mut client = Client::from_config(irc::client::prelude::Config {
            owners: vec![String::from("eyebot-rs")],
            nickname: Some(data.bot_username.clone()),
            username: Some(data.bot_username.clone()),
            server: Some(String::from("irc.chat.twitch.tv")),
            ..Default::default()
        })
        .await?;
        let stream = client.stream()?;
        let client = Arc::new(client);

        Ok(ChatClient {
            sender: watch::channel(ChatMessage {
                client: client.clone(),
                channel: String::new(),
                message: String::new(),
            })
            .0,
            data,
            stream,
            client,
        })
    }

    pub fn on_chat<Fut: Future>(
        &self,
        mut f: impl FnMut(ChatMessage) -> Fut,
    ) -> impl Future<Output = ()> {
        let mut receiver = self.sender.subscribe();
        async move {
            while receiver.changed().await.is_ok() {
                let chat_message = receiver.borrow().clone();
                f(chat_message).await;
            }
        }
    }
    pub async fn run(self) -> Result<(), ChatClientError> {
        self.handle_auth_messages()
            .await?
            .handle_join_messages()
            .await?
            .handle_chat_messages()
            .await?;

        Ok(())
    }

    async fn handle_chat_messages(mut self) -> Result<(), ChatClientError> {
        while let Some(message) = self.stream.next().await.transpose()? {
            match message.command {
                Command::PING(part1, part2) => self.client.send(Command::PONG(part1, part2))?,
                Command::PONG(_, _) => (),

                Command::PRIVMSG(_, message) => {
                    // TODO: stop sending on error
                    let _ = self.sender.send(ChatMessage {
                        client: self.client.clone(),
                        channel: self.data.chat_channel.clone(),
                        message,
                    });
                }

                Command::Raw(comm, _) if comm == "USERSTATE" => (),

                // _ => println!("unknown message: {:?}", message.command),
                _ => return Err(ChatClientError::ChatUnrecognized(message)),
            }
        }
        unreachable!("Chat connection closed")
    }

    async fn handle_auth_messages(mut self) -> Result<Self, ChatClientError> {
        self.client.send(Command::CAP(
            None,
            irc::proto::CapSubCommand::REQ,
            None,
            Some(String::from(
                "twitch.tv/membership twitch.tv/tags twitch.tv/commands",
            )),
        ))?;
        self.client.send(Command::PASS(format!(
            "oauth:{}",
            self.data.access.get_credentials().await?.access_token
        )))?;
        self.client
            .send(Command::NICK(self.data.bot_username.clone()))?;

        #[derive(Default)]
        struct Memory {
            ack: bool,
            welcome: bool,
            yourhost: bool,
            created: bool,
            myinfo: bool,
            motdstart: bool,
            motd: bool,
            endofmotd: bool,
            globaluserstate: bool,
        }
        let mut memory = Memory::default();

        while let Some(message) = self.stream.next().await.transpose()? {
            match message.command {
                Command::NOTICE(_, message) => return Err(ChatClientError::AuthError(message)),
                Command::PING(part1, part2) => self.client.send(Command::PONG(part1, part2))?,
                Command::PONG(_, _) => (),

                Command::CAP(Some(_), irc::proto::CapSubCommand::ACK, Some(_), None) => {
                    memory.ack = true;
                }
                Command::Response(response, _) => match response {
                    Response::RPL_WELCOME => memory.welcome = true,
                    Response::RPL_YOURHOST => memory.yourhost = true,
                    Response::RPL_CREATED => memory.created = true,
                    Response::RPL_MYINFO => memory.myinfo = true,
                    Response::RPL_MOTDSTART => memory.motdstart = true,
                    Response::RPL_MOTD => memory.motd = true,
                    Response::RPL_ENDOFMOTD => memory.endofmotd = true,
                    _ => return Err(ChatClientError::AuthUnrecognized(message)),
                },

                // TODO: handle states
                Command::Raw(comm, _) if comm == "GLOBALUSERSTATE" => memory.globaluserstate = true,

                _ => return Err(ChatClientError::AuthUnrecognized(message)),
            }

            if memory.ack
                && memory.welcome
                && memory.yourhost
                && memory.created
                && memory.myinfo
                && memory.motdstart
                && memory.motd
                && memory.endofmotd
                && memory.globaluserstate
            {
                return Ok(self);
            }
        }

        Err(ChatClientError::AuthIncomplete)
    }
    async fn handle_join_messages(mut self) -> Result<Self, ChatClientError> {
        self.client.send(Command::JOIN(
            format!("#{}", self.data.chat_channel),
            None,
            None,
        ))?;

        #[derive(Default)]
        struct Memory {
            join: bool,
            namreply: bool,
            endofnames: bool,
            userstate: bool,
            roomstate: bool,
        }
        let mut memory = Memory::default();

        while let Some(message) = self.stream.next().await.transpose()? {
            match message.command {
                Command::NOTICE(_, message) => return Err(ChatClientError::JoinError(message)),
                Command::PING(part1, part2) => self.client.send(Command::PONG(part1, part2))?,
                Command::PONG(_, _) => (),

                Command::JOIN(_, _, _) => memory.join = true,

                Command::Response(response, _) => match response {
                    Response::RPL_NAMREPLY => memory.namreply = true,
                    Response::RPL_ENDOFNAMES => memory.endofnames = true,

                    _ => return Err(ChatClientError::JoinUnrecognized(message)),
                },

                // TODO: handle states
                Command::Raw(comm, _) if comm == "USERSTATE" => memory.userstate = true,
                Command::Raw(comm, _) if comm == "ROOMSTATE" => memory.roomstate = true,

                _ => return Err(ChatClientError::JoinUnrecognized(message)),
            }

            if memory.join
                && memory.namreply
                && memory.endofnames
                && memory.userstate
                && memory.roomstate
            {
                return Ok(self);
            }
        }

        Err(ChatClientError::JoinIncomplete)
    }
}
