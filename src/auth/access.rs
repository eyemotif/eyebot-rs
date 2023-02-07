use super::creds::{Credentials, OAuthToken};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::any::TypeId;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct AccessTokenManager {
    creds: Arc<RwLock<Credentials>>,
    client_id: Arc<String>,
    client_secret: Arc<String>,
}

#[derive(Debug, Clone)]
pub struct AccessTokenManagerData {
    pub oauth: OAuthToken,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

#[derive(Debug)]
pub enum AccessTokenManagerError {
    Net(reqwest::Error),
    BadData(serde_json::Error),
    OnRequest(TwitchError),
    OnValidate(TwitchError),
    OnRefresh(TwitchError),
    InvalidValidateResponse,
}

#[derive(Debug, Deserialize)]
pub struct TwitchError {
    pub error: Option<String>,
    pub status: u16,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct TokenRequestResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct TokenValidationResponse {
    client_id: String,
    expires_in: u64,
}

impl AccessTokenManager {
    pub async fn new(data: AccessTokenManagerData) -> Result<Self, AccessTokenManagerError> {
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
            oauth: data.oauth,
            access_token: response.access_token,
            refresh_token: response.refresh_token,
        }));
        // TODO: deal with expires_in field
        // AccessTokenManager::spawn_daemon(creds.clone());
        Ok(AccessTokenManager {
            creds,
            client_id: Arc::new(data.client_id),
            client_secret: Arc::new(data.client_secret),
        })
    }

    pub async fn validate(&self) -> Result<bool, AccessTokenManagerError> {
        let creds = self.creds.read().unwrap();
        let response = reqwest::Client::new()
            .get("https://id.twitch.tv/oauth2/validate")
            .header("Authorization", format!("OAuth {}", creds.access_token))
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
            Err(AccessTokenManagerError::OnRequest(TwitchError {
                error: _,
                status: 401,
                message,
            })) if message == "invalid access token" => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub async fn refresh(&self) -> Result<(), AccessTokenManagerError> {
        let mut creds = self.creds.write().unwrap();
        let response = reqwest::Client::new()
            .post(
                String::from("https://id.twitch.tv/oauth2/token?grant_type=refresh_token")
                    + &format!(
                        "refresh_token={}&client_id={}&client_secret={}",
                        creds.refresh_token, self.client_id, self.client_secret
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

        creds.access_token = response.access_token;
        creds.refresh_token = response.refresh_token;

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

    // fn spawn_daemon(data: Arc<RwLock<Credentials>>) {
    //     tokio::spawn(async move {});
    // }

    fn parse_twitch<T: DeserializeOwned + 'static>(
        data: &str,
        major_err: impl FnOnce(TwitchError) -> AccessTokenManagerError,
    ) -> Result<T, AccessTokenManagerError> {
        match serde_json::from_str(&data) {
            Ok(data) => Ok(data),
            Err(err) => match serde_json::from_str::<TwitchError>(&data) {
                Ok(err) => Err(major_err(err)),
                Err(_) => Err(AccessTokenManagerError::BadData(err)),
            },
        }
    }
}
