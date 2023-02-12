use super::creds::Credentials;
use super::error::AccessTokenManagerError;
use super::{AccessTokenManagerOAuth, AccessTokenManagerTokens};
use crate::twitch::TwitchError;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct AccessTokenManager {
    creds: Arc<RwLock<Credentials>>,
    client_id: Arc<String>,
    client_secret: Arc<String>,
    token_store: PathBuf,
}

#[derive(Debug, Deserialize)]
struct TokenRequestResponse {
    access_token: String,
    refresh_token: String,
    // expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct TokenValidationResponse {
    client_id: String,
    // expires_in: u64,
}

impl AccessTokenManager {
    #[must_use]
    pub async fn new_oauth(data: AccessTokenManagerOAuth) -> Result<Self, AccessTokenManagerError> {
        let client = reqwest::Client::new();
        let response = client
            .post(
                // cargo fmt doesn't format huge strings
                String::from("https://id.twitch.tv/oauth2/token?grant_type=authorization_code&")
                    + &format!(
                        "client_id={}&client_secret={}&code={}&redirect_uri={}",
                        data.client_id, data.client_secret, data.oauth.0, data.redirect_url
                    ),
            )
            .send()
            .await
            .map_err(AccessTokenManagerError::Net)?
            .text()
            .await
            .map_err(AccessTokenManagerError::Net)?;
        let response = AccessTokenManager::parse_twitch::<TokenRequestResponse>(
            &response,
            AccessTokenManagerError::OnRequest,
        )?;

        let creds = Arc::new(RwLock::new(Credentials {
            oauth: Some(data.oauth),
            access_token: response.access_token,
            refresh_token: response.refresh_token,
        }));
        // TODO: deal with expires_in field
        // AccessTokenManager::spawn_daemon(creds.clone());
        let manager = AccessTokenManager {
            creds,
            client_id: Arc::new(data.client_id),
            client_secret: Arc::new(data.client_secret),
            token_store: data.tokens_store_path,
        };
        manager.write_tokens()?;
        Ok(manager)
    }

    #[must_use]
    pub async fn new_tokens(
        data: AccessTokenManagerTokens,
    ) -> Result<Self, AccessTokenManagerError> {
        let tokens = if data.tokens_store_path.try_exists()? {
            std::fs::read_to_string(&data.tokens_store_path)?
        } else {
            return Err(AccessTokenManagerError::InvalidTokens);
        };

        let (access_token, refresh_token) = tokens
            .trim()
            .split_once(' ')
            .map(|(a, b)| (String::from(a), String::from(b)))
            .ok_or(AccessTokenManagerError::IO(
                std::io::ErrorKind::InvalidData.into(),
            ))?;

        let creds = Arc::new(RwLock::new(Credentials {
            oauth: None,
            access_token,
            refresh_token,
        }));
        let manager = AccessTokenManager {
            creds,
            client_id: Arc::new(data.client_id),
            client_secret: Arc::new(data.client_secret),
            token_store: data.tokens_store_path,
        };
        if manager.validate().await? {
            Ok(manager)
        } else {
            Err(AccessTokenManagerError::InvalidTokens)
        }
    }

    pub async fn validate(&self) -> Result<bool, AccessTokenManagerError> {
        let response = reqwest::Client::new()
            .get("https://id.twitch.tv/oauth2/validate")
            .header(
                "Authorization",
                format!("OAuth {}", self.creds.read().unwrap().access_token),
            )
            .send()
            .await
            .map_err(AccessTokenManagerError::Net)?
            .text()
            .await
            .map_err(AccessTokenManagerError::Net)?;

        match AccessTokenManager::parse_twitch::<TokenValidationResponse>(
            &response,
            AccessTokenManagerError::OnValidate,
        ) {
            Ok(response) => {
                if response.client_id == *self.client_id {
                    Ok(true)
                } else {
                    Err(AccessTokenManagerError::InvalidValidateResponse)
                }
            }
            Err(AccessTokenManagerError::OnValidate(TwitchError {
                error: _,
                status: 401,
                message,
            })) if message == "invalid access token" => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub async fn refresh(&self) -> Result<(), AccessTokenManagerError> {
        let response = reqwest::Client::new()
            .post(
                String::from("https://id.twitch.tv/oauth2/token?grant_type=refresh_token")
                    + &format!(
                        "refresh_token={}&client_id={}&client_secret={}",
                        self.creds.read().unwrap().refresh_token,
                        self.client_id,
                        self.client_secret
                    ),
            )
            .send()
            .await
            .map_err(AccessTokenManagerError::Net)?
            .text()
            .await
            .map_err(AccessTokenManagerError::Net)?;
        let response = AccessTokenManager::parse_twitch::<TokenRequestResponse>(
            &response,
            AccessTokenManagerError::OnRefresh,
        )?;

        let mut creds = self.creds.write().unwrap();
        creds.access_token = response.access_token;
        creds.refresh_token = response.refresh_token;

        drop(creds);
        self.write_tokens()?;

        Ok(())
    }

    pub fn read_credentials_unvalidated(&self) -> std::sync::RwLockReadGuard<'_, Credentials> {
        self.creds.read().unwrap()
    }
    pub async fn get_credentials(&self) -> Result<Credentials, AccessTokenManagerError> {
        if self.validate().await? {
            Ok(self.read_credentials_unvalidated().clone())
        } else {
            self.refresh().await?;
            Ok(self.read_credentials_unvalidated().clone())
        }
    }

    fn write_tokens(&self) -> Result<(), AccessTokenManagerError> {
        let creds = self.creds.read().unwrap();
        std::fs::write(
            &self.token_store,
            format!("{} {}", creds.access_token, creds.refresh_token),
        )?;
        Ok(())
    }

    fn parse_twitch<T: DeserializeOwned + 'static>(
        data: &str,
        major_err: impl FnOnce(TwitchError) -> AccessTokenManagerError,
    ) -> Result<T, AccessTokenManagerError> {
        match serde_json::from_str(data) {
            Ok(data) => Ok(data),
            Err(err) => match serde_json::from_str::<TwitchError>(data) {
                Ok(err) => Err(major_err(err)),
                Err(_) => Err(AccessTokenManagerError::BadData(err)),
            },
        }
    }
}

impl std::fmt::Display for AccessTokenManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessTokenManagerError::Net(err) => {
                f.write_fmt(format_args!("Error sending a request to Twitch: {err}"))
            }

            AccessTokenManagerError::BadData(err) => {
                f.write_fmt(format_args!("Error parsing a response from Twitch: {err}"))
            }
            AccessTokenManagerError::OnRequest(err) => f.write_fmt(format_args!(
                "Error {} requesting an Access Token from Twitch: {}",
                err.status, err.message
            )),
            AccessTokenManagerError::OnValidate(err) => f.write_fmt(format_args!(
                "Error {} validating an Access Token: {}",
                err.status, err.message
            )),
            AccessTokenManagerError::OnRefresh(err) => f.write_fmt(format_args!(
                "Error {} refreshing an Access Token: {}",
                err.status, err.message
            )),
            AccessTokenManagerError::InvalidValidateResponse => f.write_str(
                "The Client Id given in a token validation did not match the given Client Id.",
            ),
            AccessTokenManagerError::InvalidTokens => {
                f.write_str("The given Access and/or Refresh Tokens were invalid.")
            }
            AccessTokenManagerError::IO(err) => f.write_fmt(format_args!(
                "Error accessing the Access/Refresh Tokens' store file': {err}",
            )),
        }
    }
}
impl std::error::Error for AccessTokenManagerError {}
impl From<std::io::Error> for AccessTokenManagerError {
    fn from(value: std::io::Error) -> Self {
        AccessTokenManagerError::IO(value)
    }
}
