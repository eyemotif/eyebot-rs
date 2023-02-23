use super::component;
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
        component_kind: component::Type,
    },
    PlayAudio {
        data: Vec<Vec<component::Sound>>,
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
#[serde(deny_unknown_fields)]
pub struct Response {
    pub(super) state: String,
    pub(super) tag: MessageTag,
    #[serde(flatten)]
    pub data: ResponseData,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "payload")]
#[serde(deny_unknown_fields)]
pub enum ResponseData {
    Ok,
    Data { payload: String },
    Error { is_internal: bool, message: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct MessageTag(#[serde(with = "serde_arc_str")] pub(super) std::sync::Arc<String>);

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
    pub(super) fn clone(&self) -> Self {
        MessageTag(self.0.clone())
    }
}
