use crate::twitch::TwitchError;

#[derive(Debug)]
pub enum EventsubError {
    Access(crate::auth::error::AccessTokenManagerError),
    OnConnect(tokio_tungstenite::tungstenite::Error),
    OnReconnect(tokio_tungstenite::tungstenite::Error),
    OnPong(std::io::Error),
    OnWelcome(tokio_tungstenite::tungstenite::Error),
    WelcomeInvalid,
    WelcomeIncomplete,
    OnOutbound(reqwest::Error),
    Twitch(TwitchError),
    OnReceive(tokio_tungstenite::tungstenite::Error),
    ReceiveInvalid,
}

impl std::fmt::Display for EventsubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventsubError::Access(error) => f.write_fmt(format_args!(
                "Eventsub error while trying to get an Access Token: {error}",
            )),
            EventsubError::OnConnect(err) => f.write_fmt(format_args!(
                "Eventsub error while connecting to Twitch: {err}"
            )),
            EventsubError::OnReconnect(err) => f.write_fmt(format_args!(
                "Eventsub error while reconnecting to Twitch: {err}"
            )),
            EventsubError::OnWelcome(err) => f.write_fmt(format_args!(
                "Eventsub error while receiving a Welcome message from Twitch: {err}"
            )),
            EventsubError::WelcomeInvalid => f.write_str("Eventsub: Invalid Welcome response"),
            EventsubError::WelcomeIncomplete => {
                f.write_fmt(format_args!("Eventsub: Missing Welcome response"))
            }
            EventsubError::OnPong(err) => f.write_fmt(format_args!(
                "Eventsub error while sending a Pong message: {err}"
            )),
            EventsubError::OnOutbound(err) => f.write_fmt(format_args!(
                "Eventsub error while sending data to Twitch: {err}"
            )),
            EventsubError::Twitch(err) => f.write_fmt(format_args!(
                "Eventsub error while sending data to Twitch: {err}"
            )),
            EventsubError::OnReceive(err) => f.write_fmt(format_args!(
                "Eventsub error while receiving a message from Twitch: {err}"
            )),
            EventsubError::ReceiveInvalid => {
                f.write_str("Eventsub: Message received was not valid JSON")
            }
        }
    }
}
impl std::error::Error for EventsubError {}
