use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub oauth: Option<OAuthToken>,
    pub access_token: String,
    pub(super) refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct OAuthToken(pub String);

#[derive(Debug)]
pub struct AccessTokenManagerOAuth {
    pub oauth: OAuthToken,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub tokens_store_path: PathBuf,
}
#[derive(Debug)]
pub struct AccessTokenManagerTokens {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub tokens_store_path: PathBuf,
}

#[derive(Debug)]
pub struct OAuthServerData {
    pub client_id: String,
    pub host_address: String,
    pub response_path: String,
    pub scopes: Vec<String>,
}
