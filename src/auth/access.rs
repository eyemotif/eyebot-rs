//! Interface to handle Twitch's OAuth-driven authentification.
use super::creds::Credentials;
use super::error::AccessTokenManagerError;
use super::{AccessTokenManagerOAuth, AccessTokenManagerTokens};
use crate::twitch::TwitchError;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// The data to provide Access Tokens to various Twitch interfaces.
///
/// Can be reused by cloning.
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
    /// Creates a new `TokenRequestResponse` using an OAuth token to create new
    /// Access and Refresh tokens.
    ///
    /// # Errors
    /// Returns `Err(AccessTokenManagerError...)`:
    /// * `::Net` if a response was not received from Twitch.
    /// * `::OnRequest` if Twitch denied the request to create new Access
    /// and Refresh tokens.
    /// * `::IO` if the Access and Refresh tokens were not written to Disk.
    /// * `::BadData` if a response from Twitch could not be parsed.
    pub async fn new_oauth(data: AccessTokenManagerOAuth) -> Result<Self, AccessTokenManagerError> {
        let client = reqwest::Client::new();
        let response = client
            .post(
                // cargo fmt doesn't format huge strings
                String::from("https://id.twitch.tv/oauth2/token?grant_type=authorization_code")
                    + &format!(
                        "&client_id={}&client_secret={}&code={}&redirect_uri={}",
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

    /// Creates a new `TokenRequestResponse` using existing Access and Refresh
    /// tokens to validate and refresh them.
    ///
    /// # Errors
    /// Returns `Err(AccessTokenManagerError...)`:
    /// * `::InvalidTokens` if the Access and Refresh do not exist on Disk.
    /// * `::IO` if the Access and Refresh tokens were not read from or written
    ///   to Disk, or could not be parsed from the file on
    /// * `::Net` if a response was not received from Twitch.
    /// * `::OnValidate` if Twitch denied the request to validate the Access and
    ///   Refresh tokens.
    /// * `::InvalidValidateResponse` if Twitch sent a malformed response to the
    ///   validation request.
    /// * `::OnRefresh` if Twitch denied the request to refresh the Access and
    ///   Refresh tokens.
    /// * `::BadData` if a response from Twitch could not be parsed.
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

        if !manager.validate().await? {
            manager.refresh().await?;
        }
        Ok(manager)
    }

    /// Sends a validation request to Twitch. Returns `Ok(true)` if the stored Access
    /// and Refresh tokens are valid, and `Ok(false)` if they are not.
    ///
    /// # Errors
    /// Returns `Err(AccessTokenManagerError...)`:
    /// * `::Net` if a response was not received from Twitch.
    /// * `::OnValidate` if Twitch denied the request to validate the Access and
    ///   Refresh tokens.
    /// * `::InvalidValidateResponse` if Twitch sent a malformed response to the
    ///   validation request.
    /// * `::BadData` if a response from Twitch could not be parsed.
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

    /// Sends a refresh request to Twitch.
    ///
    /// # Errors
    /// Returns `Err(AccessTokenManagerError...)`:
    /// * `::Net` if a response was not received from Twitch.
    /// * `::OnRefresh` if Twitch denied the request to refresh the Access and
    ///   Refresh tokens.
    /// * `::BadData` if a response from Twitch could not be parsed.
    pub async fn refresh(&self) -> Result<(), AccessTokenManagerError> {
        let response = reqwest::Client::new()
            .post(
                String::from("https://id.twitch.tv/oauth2/token?grant_type=refresh_token")
                    + &format!(
                        "&refresh_token={}&client_id={}&client_secret={}",
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

    /// Gives a direct [RwLockReadGuard](std::sync::RwLockReadGuard) to the [Credentials] stored internally.
    ///
    /// Note that these [Credentials] are not guaranteed to be valid, and as
    /// long as the [RwLockReadGuard](std::sync::RwLockReadGuard) is held, they
    /// can never be refreshed.
    pub fn read_credentials_unvalidated(&self) -> std::sync::RwLockReadGuard<'_, Credentials> {
        self.creds.read().unwrap()
    }
    /// Gives a valid copy of the internal [Credentials]. The credentials
    /// returned are guaranteed to be valid at the moment they are returned.
    ///
    /// If the internal [Credentials] are not valid at the moment this method is
    /// called, they will be refreshed, and the refreshed [Credentials] will be returned.
    ///
    /// # Errors
    /// * `::Net` if a response was not received from Twitch.
    /// * `::OnValidate` if Twitch denied the request to validate the Access and
    ///   Refresh tokens.
    /// * `::InvalidValidateResponse` if Twitch sent a malformed response to the
    ///   validation request.
    /// * `::OnRefresh` if Twitch denied the request to refresh the Access and
    ///   Refresh tokens.
    /// * `::BadData` if a response from Twitch could not be parsed.
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
