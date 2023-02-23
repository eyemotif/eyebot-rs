use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
pub enum Type {
    Audio,
}

#[derive(Debug, Serialize)]
pub struct Sound {
    pub name: String,
}
