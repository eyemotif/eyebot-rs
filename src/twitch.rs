use crate::auth::access::AccessTokenManager;
use reqwest::Client;
use ring::rand::SecureRandom;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
pub struct HelixAuth {
    pub client_id: String,
    pub access: AccessTokenManager,
}

#[derive(Debug, Deserialize)]
pub struct TwitchError {
    pub error: Option<String>,
    pub status: u16,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TwitchUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct TwitchEmote {
    pub id: String,
    pub name: String,
    pub format: Vec<String>,
    pub scale: Vec<String>,
    pub theme_mode: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TwitchChannel {
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    pub broadcaster_language: String,
    pub game_id: String,
    pub game_name: String,
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TwitchStream {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_id: String,
    pub game_name: String,
    pub title: String,
    pub tags: Vec<String>,
    pub viewer_count: u32,
    pub started_at: String,
    pub language: String,
    pub is_mature: bool,
}

#[derive(Debug, Deserialize)]
pub struct TwitchBadgeUrls {
    pub id: String,
    pub image_url_1x: String,
    pub image_url_2x: String,
    pub image_url_4x: String,
}

#[derive(Debug, Deserialize)]
struct BadgesResponse {
    pub set_id: String,
    pub versions: Vec<TwitchBadgeUrls>,
}

pub fn from_twitch_response<T: serde::de::DeserializeOwned>(twitch_response: &str) -> Result<T> {
    if let Ok(error) = serde_json::from_str::<TwitchError>(twitch_response) {
        Err(error.into())
    } else {
        Ok(serde_json::from_str(twitch_response)?)
    }
}

pub async fn user_from_login(login: &str, auth: &HelixAuth) -> Result<Option<TwitchUser>> {
    get_paginated_value(
        format!("https://api.twitch.tv/helix/users?login={login}"),
        auth,
    )
    .await
}
pub async fn user_from_id(id: &str, auth: &HelixAuth) -> Result<Option<TwitchUser>> {
    get_paginated_value(format!("https://api.twitch.tv/helix/users?id={id}"), auth).await
}
pub async fn channel(broadcaster_id: &str, auth: &HelixAuth) -> Result<Option<TwitchChannel>> {
    get_paginated_value(
        format!("https://api.twitch.tv/helix/channels?broadcaster_id={broadcaster_id}'"),
        auth,
    )
    .await
}
pub async fn stream_from_user_id(user_id: &str, auth: &HelixAuth) -> Result<Option<TwitchStream>> {
    get_paginated_value(
        format!("https://api.twitch.tv/helix/streams?user_id={user_id}"),
        auth,
    )
    .await
}

pub async fn get_global_badges(auth: &HelixAuth) -> Result<HashMap<String, Vec<TwitchBadgeUrls>>> {
    Ok(
        get_paginated_values("https://api.twitch.tv/helix/chat/badges/global", auth)
            .await?
            .into_iter()
            .map(|value| serde_json::from_value::<BadgesResponse>(value))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .map(|response| (response.set_id, response.versions))
            .collect(),
    )
}
pub async fn get_channel_badges(
    broadcaster_id: &str,
    auth: &HelixAuth,
) -> Result<HashMap<String, Vec<TwitchBadgeUrls>>> {
    Ok(get_paginated_values(
        format!("https://api.twitch.tv/helix/chat/badges?broadcaster_id={broadcaster_id}"),
        auth,
    )
    .await?
    .into_iter()
    .map(|value| serde_json::from_value::<BadgesResponse>(value))
    .collect::<std::result::Result<Vec<_>, _>>()?
    .into_iter()
    .map(|response| (response.set_id, response.versions))
    .collect())
}

pub async fn get_all_badges(
    broadcaster_id: &str,
    auth: &HelixAuth,
) -> Result<HashMap<String, Vec<TwitchBadgeUrls>>> {
    let (global, channel) = tokio::try_join!(
        get_global_badges(auth),
        get_channel_badges(broadcaster_id, auth),
    )?;
    Ok(global.into_iter().chain(channel.into_iter()).collect())
}

pub fn random_chatter_color() -> String {
    lazy_static::lazy_static!(
        static ref RNG: ring::rand::SystemRandom = ring::rand::SystemRandom::new();
    );
    let mut color = [0u8; 3];
    loop {
        match RNG.fill(&mut color) {
            Ok(()) => break,
            Err(_) => (),
        }
    }
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}

async fn get_paginated_values<U: reqwest::IntoUrl>(
    url: U,
    auth: &HelixAuth,
) -> Result<Vec<serde_json::Value>> {
    let response = Client::new()
        .get(url)
        .header("Client-Id", &auth.client_id)
        .header(
            "Authorization",
            format!(
                "Bearer {}",
                auth.access.get_credentials().await?.access_token
            ),
        )
        .send()
        .await?
        .text()
        .await?;

    let json = from_twitch_response::<Value>(&response)?;
    Ok(json
        .as_object()
        .ok_or("Expected object")?
        .get("data")
        .ok_or("Expected field data")?
        .as_array()
        .ok_or("Expected array")?
        .clone())
}

async fn get_paginated_value<T: serde::de::DeserializeOwned, U: reqwest::IntoUrl>(
    url: U,
    auth: &HelixAuth,
) -> Result<Option<T>> {
    let arr = get_paginated_values(url, auth).await?;

    Ok(arr
        .get(0)
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()?)
}

impl std::fmt::Display for TwitchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = if let Some(error) = &self.error {
            format!(" {error}")
        } else {
            String::new()
        };
        f.write_fmt(format_args!(
            "Twitch error {}{}: {}",
            self.status, error, self.message,
        ))
    }
}
impl std::error::Error for TwitchError {}

impl PartialEq for TwitchUser {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for TwitchUser {}

impl PartialEq for TwitchEmote {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for TwitchEmote {}
