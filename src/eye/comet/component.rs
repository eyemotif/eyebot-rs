use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Audio,
}

#[derive(Debug, Serialize)]
pub struct Sound {
    pub name: String,
}

impl Sound {
    pub fn parse(input: &str) -> Vec<Vec<Sound>> {
        input
            .trim()
            .split([' ', ','])
            .map(|word| {
                word.split('+')
                    .map(|sound| Sound {
                        name: String::from(sound),
                    })
                    .collect()
            })
            .collect()
    }
}
