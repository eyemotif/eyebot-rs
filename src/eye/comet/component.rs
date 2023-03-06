use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Audio,
    Chat,
}

#[derive(Debug, Serialize, Clone)]
pub struct Sound {
    pub name: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Chat {
    Text { content: String },
    Emote { emote: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatterInfo {
    pub display_name: String,
    pub name_color: String,
    pub badges: Vec<String>,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChatMetadata {
    None,
    Action,
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

impl Chat {
    pub fn from_chat_message(
        chat_message: &crate::chat::data::ChatMessage,
    ) -> (Vec<Chat>, ChatMetadata) {
        let (text, metadata) = match chat_message.try_get_action() {
            Some(action) => (action, ChatMetadata::Action),
            None => (chat_message.text.as_str(), ChatMetadata::None),
        };
        let mut output = Vec::new();
        let mut current_chat = String::new();

        'char: for (i, c) in text.chars().enumerate() {
            let i = i as u16;
            for emote in &chat_message.emotes {
                for (start, end) in &emote.locations {
                    let (start, end) = (*start, *end);

                    if (start..=end).contains(&i) {
                        if i == start {
                            if !current_chat.is_empty() {
                                output.push(Chat::Text {
                                    content: current_chat,
                                });
                                current_chat = String::new();
                            }
                            output.push(Chat::Emote {
                                emote: emote.id.clone(),
                            });
                        }

                        continue 'char;
                    }
                }
            }

            current_chat.push(c);
        }

        if !current_chat.is_empty() {
            output.push(Chat::Text {
                content: current_chat,
            });
        }

        (output, metadata)
    }
}
