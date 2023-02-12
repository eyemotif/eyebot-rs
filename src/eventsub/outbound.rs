use super::error::EventsubError;
use super::subscription::Subscription;
use crate::twitch::HelixAuth;
use reqwest::Client;
use serde_json::Value;

// FIXME: this only works when subscribing to notifications on the same channel
// as the auth
pub async fn send_subscriptions(
    subscriptions: &[Subscription],
    session_id: &str,
    auth: &HelixAuth,
) -> Result<(), EventsubError> {
    let access_token = auth
        .access
        .get_credentials()
        .await
        .map_err(EventsubError::Access)?
        .access_token;
    for subscription in subscriptions {
        let mut outbound = serde_json::Map::from_iter(
            [
                ("version", Value::String(String::from("1"))),
                (
                    "transport",
                    serde_json::json!({
                        "method": "websocket",
                        "session_id": session_id,
                    }),
                ),
            ]
            .map(|(k, v)| (String::from(k), v)),
        );
        for (k, v) in serde_json::to_value(subscription.clone())
            .expect("Subscription is always serializable")
            .as_object()
            .expect("Subscription is always an object")
        {
            outbound.insert(k.clone(), v.clone());
        }

        let client = Client::new();
        let response = client
            .post("https://api.twitch.tv/helix/eventsub/subscriptions")
            .header("Content-Type", "application/json")
            .header("Client-Id", &auth.client_id)
            .header("Authorization", format!("Bearer {access_token}"))
            .body(
                serde_json::to_string(&Value::Object(outbound))
                    .expect("Value::Object always succeeds in serde_json::to_string"),
            )
            .send()
            .await
            .map_err(EventsubError::OnOutbound)?
            .text()
            .await
            .map_err(EventsubError::OnOutbound)?;
        if let Ok(twitch_error) = serde_json::from_str::<crate::twitch::TwitchError>(&response) {
            return Err(EventsubError::Twitch(twitch_error));
        }
    }
    Ok(())
}
