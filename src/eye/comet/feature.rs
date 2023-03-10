use super::interface::CometInterface;
use super::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Chat,
}

impl Feature {
    pub async fn get_features(interface: CometInterface) -> Option<HashSet<Feature>> {
        // FIXME: this deadlocks sometimes
        // For some reason the async block never gets entered? I don't get it
        let response = interface.send_message(Message::Features {}).await?;

        match response {
            super::ResponseData::Ok => unreachable!(),
            super::ResponseData::Data { payload } => {
                serde_json::from_str(&payload).expect("Data should always be a list of Features")
            }
            super::ResponseData::Error { .. } => unreachable!(),
        }
    }

    pub async fn init(
        interface: CometInterface,
        features: HashSet<Feature>,
        streamer_username: String,
    ) -> Option<Result<(), String>> {
        for feature in &features {
            match feature {
                Feature::Chat => match interface
                    .send_message(Message::ChatSetEmotes {
                        username: streamer_username.clone(),
                    })
                    .await?
                {
                    super::ResponseData::Ok => (),
                    super::ResponseData::Data { .. } => unreachable!(),
                    super::ResponseData::Error { message, .. } => return Some(Err(message)),
                },
            }
        }

        interface.set_features(features).await;

        Some(Ok(()))
    }
}
