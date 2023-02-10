use super::subscription::Subscription;
use reqwest::Client;
use serde_json::Value;

pub async fn send_subscriptions(
    subscriptions: &[Subscription],
    session_id: &str,
) -> reqwest::Result<()> {
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
        client
            .post("https://api.twitch.tv/helix/eventsub/subscriptions")
            .body(
                serde_json::to_string(&Value::Object(outbound))
                    .expect("Value::Object always succeeds in serde_json::to_string"),
            )
            .send()
            .await?;
    }
    Ok(())
}
