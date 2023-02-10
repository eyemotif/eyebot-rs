use crate::auth::access::AccessTokenManager;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
}

pub fn from_twitch_response<T: serde::de::DeserializeOwned>(twitch_response: &str) -> Result<T> {
    if let Ok(error) = serde_json::from_str::<TwitchError>(twitch_response) {
        Err(error.into())
    } else {
        Ok(serde_json::from_str(twitch_response)?)
    }
}

pub async fn user_from_login(login: &str, auth: &HelixAuth) -> Result<Option<TwitchUser>> {
    user_from_url(
        format!("https://api.twitch.tv/helix/users?login={login}'"),
        auth,
    )
    .await
}
pub async fn user_from_id(id: &str, auth: &HelixAuth) -> Result<Option<TwitchUser>> {
    user_from_url(format!("https://api.twitch.tv/helix/users?id={id}'"), auth).await
}

async fn user_from_url<U: reqwest::IntoUrl>(
    url: U,
    auth: &HelixAuth,
) -> Result<Option<TwitchUser>> {
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
    let maybe_user = json
        .as_object()
        .ok_or("Expected object")?
        .get("data")
        .ok_or("Expected field data")?
        .as_array()
        .ok_or("Expected array")?
        .get(0);

    Ok(maybe_user
        .map(|user| serde_json::from_value(user.clone()))
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
