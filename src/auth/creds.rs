#[derive(Debug, Clone)]
pub struct Credentials {
    pub oauth: OAuthToken,
    pub access_token: String,
    pub(super) refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct OAuthToken(pub String);

#[derive(Debug)]
pub struct AccessTokenManagerData {
    pub oauth: OAuthToken,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}
#[derive(Debug)]
pub struct OAuthClientData {
    pub client_id: String,
    pub host_address: String,
    pub response_path: String,
    pub scopes: Vec<String>,
}
