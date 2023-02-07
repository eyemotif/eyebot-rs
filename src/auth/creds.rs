#[derive(Debug, Clone)]
pub struct Credentials {
    pub oauth: OAuthToken,
    pub access_token: String,
    pub(super) refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct OAuthToken(pub(super) String);

impl OAuthToken {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
