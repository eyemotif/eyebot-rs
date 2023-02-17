use std::path::PathBuf;

/// The Access and Refresh tokens currently being managed by an
/// [`AccessTokenManager`](super::access::AccessTokenManager).
///
/// Optionally contains an [`OAuthToken`] if the other tokens were created using one.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub oauth: Option<OAuthToken>,
    pub access_token: String,
    pub(super) refresh_token: String,
}

/// A Twitch OAuth User token.
#[derive(Debug, Clone)]
pub struct OAuthToken(pub String);

/// The data needed to call [`AccessTokenManager::new_oauth`()](super::access::AccessTokenManager).
#[derive(Debug)]
pub struct AccessTokenManagerOAuth {
    /// A valid [OAuth Token](OAuthToken).
    pub oauth: OAuthToken,
    /// The app's [Client ID](https://dev.twitch.tv/docs/authentication/register-app/).
    pub client_id: String,
    /// The app's [Client Secret](https://dev.twitch.tv/docs/authentication/register-app/).
    pub client_secret: String,
    /// Any one of the app's [OAuth Redirect
    /// URLs](https://dev.twitch.tv/docs/authentication/register-app/).
    pub redirect_url: String,
    /// The path on Disk to write the Access and Refresh tokens to.
    pub tokens_store_path: PathBuf,
}
/// The data needed to call [`AccessTokenManager::new_tokens`()](super::access::AccessTokenManager).
#[derive(Debug)]
pub struct AccessTokenManagerTokens {
    /// The app's [Client ID](https://dev.twitch.tv/docs/authentication/register-app/).
    pub client_id: String,
    /// The app's [Client Secret](https://dev.twitch.tv/docs/authentication/register-app/).
    pub client_secret: String,
    /// Any one of the app's [OAuth Redirect
    /// URLs](https://dev.twitch.tv/docs/authentication/register-app/).
    pub redirect_url: String,
    /// The path on Disk to write the Access and Refresh tokens to.
    pub tokens_store_path: PathBuf,
}

/// The data needed to create an [OAuth server](super::oauth).
#[derive(Debug)]
pub struct OAuthServerData {
    /// The app's [Client ID](https://dev.twitch.tv/docs/authentication/register-app/).
    pub client_id: String,
    /// Must not contain a protocol, i.e. must be in the format of `address:port`.
    pub host_address: String,
    /// Must be in the format of `/path`.
    pub response_path: String,
    /// A list of [Scopes](https://dev.twitch.tv/docs/authentication/scopes/).
    pub scopes: Vec<String>,
}
