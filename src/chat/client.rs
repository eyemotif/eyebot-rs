use super::data::{ChatAccess, ChatMessage};
use super::error::ChatClientError;
use super::interface::ChatInterface;
use crate::chat::tag;
use futures_util::StreamExt;
use irc::client::Client;
use irc::proto::{Command, Response};
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;

#[derive(Debug)]
pub struct ChatClient {
    client: Arc<Client>,
    stream: irc::client::ClientStream,
    data: super::data::ChatClientData,
    joined_users: HashSet<String>,
    interface: super::interface::ChatInterface,
    options: crate::options::Options,
}

impl ChatClient {
    pub async fn new(
        data: super::data::ChatClientData,
        options: crate::options::Options,
    ) -> Result<Self, ChatClientError> {
        options.debug("Chat: Connecting to Twitch");

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
            joined_users: HashSet::new(),
            interface: ChatInterface::new(client.clone(), data.chat_channel.clone()),
            data,
            stream,
            client,
            options,
        })
    }

    pub fn on_chat<Fut: Future>(
        &self,
        mut f: impl FnMut(ChatMessage, ChatInterface) -> Fut,
    ) -> impl Future<Output = ()> {
        let chat_interface = self.interface.clone();
        async move {
            let mut receiver = chat_interface.0.message_channel.subscribe();
            while receiver.changed().await.is_ok() {
                let chat_message = receiver.borrow().clone();
                f(chat_message, chat_interface.clone()).await;
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

    #[must_use]
    pub fn get_interface(&self) -> ChatInterface {
        self.interface.clone()
    }
    #[must_use]
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<ChatMessage> {
        self.interface.0.message_channel.subscribe()
    }

    async fn handle_chat_messages(mut self) -> Result<(), ChatClientError> {
        self.options.debug("Chat: Ready to receive messages!");

        while let Some(message) = self.stream.next().await.transpose()? {
            match message.command {
                Command::PING(part1, part2) => self.client.send(Command::PONG(part1, part2))?,
                Command::PONG(_, _) => (),

                Command::NOTICE(_, _) => todo!(),
                Command::PRIVMSG(_, text) => {
                    let tags = tag::tags::<tag::PRIVMSGTags>(
                        &message.tags.expect("Message always has tags"),
                    )
                    .expect("Tags are always well formed");

                    // TODO: stop sending on error
                    let chat_message = ChatMessage {
                        id: tags.id,
                        channel: self.data.chat_channel.clone(),
                        text,
                        user_id: tags.user_id,
                        is_broadcaster: tags.badges.contains_key("broadcaster"),
                        is_moderator: tags.is_mod,
                        is_subscriber: tags.subscriber,
                        emotes: tags.emotes,
                        display_name: tags.display_name,
                    };

                    self.options.debug(format!(
                        "Chat: {}> {:?}",
                        chat_message.display_name, chat_message.text
                    ));

                    let _ = self.interface.0.message_channel.send(chat_message);
                }
                Command::JOIN(_, _, _) => {
                    let username = message
                        .prefix
                        .expect("The JOIN command always has a prefix");
                    let irc::proto::Prefix::Nickname(_, username, _) = username else {
                        unreachable!("The JOIN prefix is always Prefix::Nickname");
                    };
                    self.joined_users.insert(username);
                }
                Command::PART(_, _) => {
                    let username = message
                        .prefix
                        .expect("The PART command always has a prefix");
                    let irc::proto::Prefix::Nickname(_, username, _) = username else {
                        unreachable!("The PART prefix is always Prefix::Nickname");
                    };
                    self.joined_users.remove(&username);
                }

                Command::Raw(ref comm, ref _params) => {
                    match comm.as_str() {
                        "CLEARCHAT" => {
                            let tags = tag::tags::<tag::CLEARCHATTags>(
                                &message.tags.expect("Message always has tags"),
                            )
                            .expect("Tags are always well formed");
                            println!("{tags:?}");
                        }
                        "CLEARMSG" => {
                            let tags = tag::tags::<tag::CLEARMSGTags>(
                                &message.tags.expect("Message always has tags"),
                            )
                            .expect("Tags are always well formed");
                            println!("{tags:?}");
                        }
                        "HOSTTARGET" => todo!(),
                        "RECONNECT" => todo!(),
                        "ROOMSTATE" => todo!(),
                        "USERNOTICE" => {
                            let tags = tag::tags::<tag::USERNOTICETags>(
                                &message.tags.expect("Message always has tags"),
                            )
                            .expect("Tags are always well formed");
                            println!("{tags:?}");
                        }
                        // TODO: handle userstates
                        "USERSTATE" => (),
                        "WHISPER" => todo!(),
                        _ => return Err(ChatClientError::ChatUnrecognized(message)),
                    }
                    // println!("USERSTATE: {:?}", message.tags);
                }

                // _ => println!("unknown message: {:?}", message),
                _ => return Err(ChatClientError::ChatUnrecognized(message)),
            }
        }
        unreachable!("Chat connection closed")
    }

    async fn handle_auth_messages(mut self) -> Result<Self, ChatClientError> {
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

        self.options.debug("Chat: Authenticating with Twitch IRC");

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
            match &self.data.access {
                ChatAccess::Authorization(access) => access.get_credentials().await?.access_token,
                ChatAccess::Implicit(access) => access.clone(),
            }
        )))?;
        self.client
            .send(Command::NICK(self.data.bot_username.clone()))?;

        let mut memory = Memory::default();

        while let Some(message) = self.stream.next().await.transpose()? {
            match message.command {
                Command::NOTICE(_, message) => return Err(ChatClientError::AuthError(message)),
                Command::PING(part1, part2) => self.client.send(Command::PONG(part1, part2))?,
                Command::PONG(_, _) => (),

                Command::CAP(Some(_), irc::proto::CapSubCommand::ACK, Some(_), None) => {
                    memory.ack = true
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
                Command::Raw(comm, _) if comm == "GLOBALUSERSTATE" => {
                    // println!("GLOBALUSERSTATE: {:?}", message.tags);
                    memory.globaluserstate = true
                }

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
        #[derive(Default)]
        struct Memory {
            join: bool,
            namreply: bool,
            endofnames: bool,
            userstate: bool,
            roomstate: bool,
        }

        self.options.debug("Chat: Joining Twitch IRC");

        self.client.send(Command::JOIN(
            format!("#{}", self.data.chat_channel),
            None,
            None,
        ))?;

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
                Command::Raw(comm, _) if comm == "USERSTATE" => {
                    // println!("SELF USERSTATE: {:?}", message.tags);
                    memory.userstate = true;
                }
                Command::Raw(comm, _) if comm == "ROOMSTATE" => {
                    // println!("ROOMSTATE: {:?}", message.tags);
                    memory.roomstate = true;
                }

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
