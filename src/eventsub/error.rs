#[derive(Debug)]
pub enum EventsubError {
    Connect(tokio_tungstenite::tungstenite::Error),
}

impl std::fmt::Display for EventsubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventsubError::Connect(err) => f.write_fmt(format_args!(
                "Eventsub error while connecting to Twitch: {err}"
            )),
        }
    }
}
impl std::error::Error for EventsubError {}
