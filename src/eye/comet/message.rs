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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub state: String,
    #[serde(flatten)]
    pub data: ResponseData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "payload")]
#[serde(deny_unknown_fields)]
pub enum ResponseData {
    Ok,
    Data { payload: String },
    Error { is_internal: bool, message: String },
}
