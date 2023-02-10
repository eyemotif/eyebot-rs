use serde::Deserialize;

#[derive(Debug)]
pub enum OAuthServerError {
    OnServerCreate(Box<dyn std::error::Error + Send + Sync>),
    OnReceive(std::io::Error),
    OnResponse(std::io::Error),
    OnAuth {
        error: String,
        error_description: String,
    },
    Ring(ring::error::Unspecified),
}

#[derive(Debug)]
pub enum AccessTokenManagerError {
    Net(reqwest::Error),
    BadData(serde_json::Error),
    OnRequest(TwitchError),
    OnValidate(TwitchError),
    OnRefresh(TwitchError),
    InvalidValidateResponse,
    InvalidTokens,
    IO(std::io::Error),
}

#[derive(Debug, Deserialize)]
pub struct TwitchError {
    pub error: Option<String>,
    pub status: u16,
    pub message: String,
}