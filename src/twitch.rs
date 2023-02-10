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

pub fn from_twitch_response<T: serde::de::DeserializeOwned>(twitch_response: &str) -> Result<T> {
    if let Ok(error) = serde_json::from_str::<TwitchError>(twitch_response) {
        Err(error.into())
    } else {
        Ok(serde_json::from_str(twitch_response)?)
    }
}

pub async fn id_from_login(login: &str, auth: &HelixAuth) -> Result<Option<String>> {
    let response = Client::new()
        .get(format!("https://api.twitch.tv/helix/users?login={login}"))
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

    Ok(maybe_user.and_then(|json| Some(json.as_object()?.get("id")?.as_str()?.to_owned())))
}
pub async fn login_from_id(id: &str, auth: &HelixAuth) -> Result<Option<String>> {
    let response = Client::new()
        .get(format!("https://api.twitch.tv/helix/users?id={id}"))
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

    Ok(maybe_user.and_then(|json| Some(json.as_object()?.get("login")?.as_str()?.to_owned())))
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
