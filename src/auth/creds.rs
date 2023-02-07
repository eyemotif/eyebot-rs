#[derive(Debug, Clone)]
pub struct Credentials {
    pub oauth: OAuthToken,
    pub access_token: String,
    pub(super) refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct OAuthToken(pub String);
