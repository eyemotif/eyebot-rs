use super::error::ChatClientError;
use irc::client::Client;
use irc::proto::Command;

#[derive(Debug)]
pub struct ChatClient {
    client: Client,
}

impl ChatClient {
    pub async fn new(data: super::data::ChatClientData) -> Result<Self, ChatClientError> {
        let client = Client::from_config(irc::client::prelude::Config {
            owners: vec![String::from("eyebot-rs")],
            nickname: Some(data.bot_username.clone()),
            username: Some(data.bot_username.clone()),
            server: Some(String::from("irc.chat.twitch.tv")),
            ..Default::default()
        })
        .await?;
        client.send(Command::CAP(
            None,
            irc::proto::CapSubCommand::REQ,
            None,
            Some(String::from(
                "twitch.tv/membership twitch.tv/tags twitch.tv/commands",
            )),
        ))?;
        client.send(Command::PASS(format!(
            "oauth:{}",
            data.access.get_credentials().await?.access_token
        )))?;
        client.send(Command::NICK(data.bot_username.clone()))?;

        Ok(ChatClient { client: client })
    }
    pub async fn handle_messages(self) -> Result<(), ChatClientError> {
        todo!()
    }
}
