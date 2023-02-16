use crate::twitch::TwitchError;

/// An Error returned by an [OAuth Server](super::oauth).
#[derive(Debug)]
pub enum OAuthServerError {
    /// An error returned when the server is first being created.
    OnServerCreate(Box<dyn std::error::Error + Send + Sync>),
    /// An error returned when the server is receiving data from Twitch.
    OnReceive(std::io::Error),
    /// An error returned when the server is sending data to Twitch.
    OnResponse(std::io::Error),
    /// An error returned if Twitch rejects the creation of an OAuth token.
    OnAuth {
        error: String,
        error_description: String,
    },
    /// An error generating random data.
    Ring(ring::error::Unspecified),
}

/// An Error returned by an [AccessTokenManager](super::access::AccessTokenManager).
#[derive(Debug)]
pub enum AccessTokenManagerError {
    /// An error returned while making a GET or POST request.
    Net(reqwest::Error),
    /// An error returned if the data from Twitch could not be deserialized.
    BadData(serde_json::Error),
    /// An error returned while requesting new Access and Refresh tokens.
    OnRequest(TwitchError),
    /// An error returned while validating Access and Refresh tokens.
    OnValidate(TwitchError),
    /// An error returned while refreshing Access and Refresh tokens.
    OnRefresh(TwitchError),
    /// An error returned if Twitch sends a malformed response while validating
    /// Access and Refresh tokens.
    InvalidValidateResponse,
    /// An error returned if the Access and Refresh tokens read from were invalid.
    InvalidTokens,
    /// An error returned while trying to read/write Access and Refresh tokens
    /// from/to the disk.
    IO(std::io::Error),
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

impl std::fmt::Display for OAuthServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthServerError::OnServerCreate(err) => f.write_fmt(format_args!(
                "Error while creating the authentification server: {err}"
            )),
            OAuthServerError::OnReceive(err) => f.write_fmt(format_args!(
                "Error while trying to receive a request to the server: {err}"
            )),
            OAuthServerError::OnResponse(err) => f.write_fmt(format_args!(
                "Error while trying to send a response from the server: {err}"
            )),
            OAuthServerError::OnAuth {
                error,
                error_description,
            } => f.write_fmt(format_args!(
                "Error {error} while validating the user's credentials: {error_description}"
            )),
            OAuthServerError::Ring(err) => {
                f.write_fmt(format_args!("Error while creating random data: {err}"))
            }
        }
    }
}
impl std::error::Error for OAuthServerError {}
