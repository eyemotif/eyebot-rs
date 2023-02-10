use crate::chat::tag;

use super::data::ChatMessage;
use super::error::ChatClientError;
use irc::client::Client;
use irc::proto::message::Tag;
use irc::proto::{Command, Response};
use std::collections::{HashMap, HashSet};
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
    joined_users: HashSet<String>,
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
            sender: watch::channel(ChatMessage::empty(client.clone())).0,
            data,
            stream,
            client,
            joined_users: HashSet::new(),
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

                Command::NOTICE(_, _) => todo!(),
                Command::PRIVMSG(_, text) => {
                    let tags = tag::tags::<tag::PRIVMSGTags>(
                        &message.tags.expect("Message always has tags"),
                    )
                    .expect("Tags are always well formed");
                    println!("CLEARCHAT {tags:?}");

                    // TODO: stop sending on error
                    let _ = self.sender.send(ChatMessage {
                        client: self.client.clone(),
                        id: tags.id,
                        channel: self.data.chat_channel.clone(),
                        text,
                        user_id: tags.user_id,
                        is_broadcaster: tags.badges.contains_key("broadcaster"),
                        is_moderator: tags.is_mod,
                        is_subscriber: tags.subscriber,
                    });
                }
                Command::JOIN(_, _, _) => {
                    let username = message
                        .prefix
                        .expect("The JOIN command always has a prefix");
                    let username = match username {
                        irc::proto::Prefix::Nickname(_, username, _) => username,
                        _ => unreachable!("The JOIN prefix is always Prefix::Nickname"),
                    };
                    self.joined_users.insert(username);
                }
                Command::PART(_, _) => {
                    let username = message
                        .prefix
                        .expect("The PART command always has a prefix");
                    let username = match username {
                        irc::proto::Prefix::Nickname(_, username, _) => username,
                        _ => unreachable!("The PART prefix is always Prefix::Nickname"),
                    };
                    self.joined_users.remove(&username);
                }

                Command::Raw(ref comm, ref params) => {
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
                Command::Raw(comm, _) if comm == "USERSTATE" => {
                    // println!("SELF USERSTATE: {:?}", message.tags);
                    memory.userstate = true
                }
                Command::Raw(comm, _) if comm == "ROOMSTATE" => {
                    // println!("ROOMSTATE: {:?}", message.tags);
                    memory.roomstate = true
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

    fn tags_to_map(tags: Vec<Tag>) -> HashMap<String, Option<String>> {
        tags.into_iter().map(|Tag(k, v)| (k, v)).collect()
    }
}
