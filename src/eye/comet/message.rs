use super::component;
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    Register {
        state: String,
    },
    GetComponents {
        #[serde(rename = "type")]
        component_type: component::Type,
    },
    PlayAudio {
        data: Vec<Vec<component::Sound>>,
    },
    AudioVolume {
        name: String,
        value: f32,
    },
    AudioClear {},
    ChatSetEmotes {
        username: String,
    },
    Chat {
        user_id: String,
        chat: Vec<component::Chat>,
        meta: component::ChatMetadata,
    },
    ChatUser {
        user_id: String,
        chat_info: component::ChatterInfo,
    },
    Features {},
    ChatClear {
        user_id: Option<String>,
    },
}

#[derive(Debug, Serialize)]
pub(super) struct TaggedMessage {
    pub state: String,
    pub tag: MessageTag,
    #[serde(flatten)]
    pub message: Message,
}

#[derive(Debug, Deserialize)]
// #[serde(deny_unknown_fields)] https://github.com/serde-rs/serde/issues/1600
pub struct Response {
    pub(super) state: String,
    pub(super) tag: MessageTag,
    #[serde(flatten)]
    pub data: ResponseData,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
pub enum ResponseData {
    Ok,
    Data { payload: String },
    Error { is_internal: bool, message: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct MessageTag(
    #[serde(with = "serde_arc_str")] pub(super) std::sync::Arc<String>,
    #[serde(skip)] bool,
);

mod serde_arc_str {
    pub fn serialize<S: serde::Serializer>(
        arc: &std::sync::Arc<String>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&*arc)
    }
    pub fn deserialize<'d, D: serde::Deserializer<'d>>(
        de: D,
    ) -> Result<std::sync::Arc<String>, D::Error> {
        let s: &str = serde::Deserialize::deserialize(de)?;

        Ok(std::sync::Arc::new(String::from(s)))
    }
}

impl MessageTag {
    pub(super) fn new() -> Self {
        lazy_static::lazy_static!(
            static ref RNG: ring::rand::SystemRandom = ring::rand::SystemRandom::new();
        );

        Self(
            std::sync::Arc::new(loop {
                let mut state = [0; 16];
                match RNG.fill(&mut state) {
                    Ok(()) => break state.into_iter().map(|byte| format!("{byte:x?}")).collect(),
                    Err(_) => (),
                };
            }),
            false,
        )
    }
    pub(super) fn close() -> Self {
        Self(std::sync::Arc::new(String::new()), true)
    }
    pub(super) fn clone(&self) -> Self {
        MessageTag(self.0.clone(), self.1)
    }
    pub(super) fn is_close(&self) -> bool {
        self.1
    }
}
