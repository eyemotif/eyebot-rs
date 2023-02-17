use crate::bot::interface::BotInterface;
use crate::chat::data::ChatMessage;
use std::collections::HashSet;

#[derive(Debug)]
pub struct CommandRules {
    pub body: Vec<CommandSection>,
    pub tags: HashSet<CommandTag>,
}
#[derive(Debug)]
pub enum CommandSection {
    Echo(String),
    ChatterName,
    WordIndex(usize),
}
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum CommandTag {
    Reply,
    Const,
    Super,
}
#[derive(Debug)]
pub enum RulesError {
    BadVariable(String),
    BadTag(String),
}

impl CommandRules {
    pub fn parse(input: &str) -> Result<Self, RulesError> {
        let mut output = CommandRules {
            body: Vec::new(),
            tags: HashSet::new(),
        };
        let mut current_word = String::new();
        let mut escape = false;

        for chr in input.chars().chain([' ']) {
            match chr {
                ' ' => {
                    if let Some(var_string) = current_word.strip_prefix('%') {
                        output.body.push(CommandRules::var_from_string(var_string)?);
                        current_word.clear();
                    } else if let Some(tag_string) = current_word.strip_prefix('&') {
                        output
                            .tags
                            .insert(CommandRules::tag_from_string(tag_string)?);
                        current_word.clear();
                    } else {
                        output.body.push(CommandSection::Echo(
                            if current_word.starts_with("\\%") || current_word.starts_with("\\&") {
                                current_word.chars().skip(2).collect()
                            } else {
                                current_word
                            },
                        ));
                        current_word = String::new();
                    }
                }
                '\\' => {
                    if escape {
                        current_word.push('\\');
                    } else {
                        escape = true;
                        continue;
                    }
                }
                chr => current_word.push(chr),
            }
            escape = false;
        }

        Ok(output)
    }

    pub fn empty_const() -> Self {
        Self {
            body: Vec::new(),
            tags: HashSet::from_iter([CommandTag::Const]),
        }
    }

    pub async fn execute(&self, args: Vec<String>, msg: &ChatMessage, bot: &BotInterface) {
        let mut chatter_name: Option<String> = None;
        let mut message = Vec::new();

        for section in &self.body {
            message.push(match section {
                CommandSection::Echo(text) => String::from(text),
                CommandSection::ChatterName => {
                    if let Some(chatter_name) = &chatter_name {
                        chatter_name.clone()
                    } else {
                        let Ok(Some(user)) = crate::twitch::user_from_id(&msg.user_id, bot.helix_auth()).await else {
                            bot.error(format!("Could not get username from id {:?}", msg.user_id)).await;
                            return;
                        };
                        chatter_name = Some(user.display_name.clone());
                        user.display_name
                    }
                }
                CommandSection::WordIndex(index) => {
                    String::from(args.get(*index).unwrap_or(&index.to_string()))
                },
            })
        }

        let message = message.join(" ");
        for tag in &self.tags {
            match tag {
                CommandTag::Reply => bot.reply(msg, message).await,
                _ => continue,
            }
            return;
        }
        bot.say(message).await;
    }

    pub fn can_run(&self, msg: &ChatMessage, _bot: &BotInterface) -> bool {
        if self.tags.contains(&CommandTag::Super) && !msg.user_is_super() {
            return false;
        }
        true
    }

    pub fn as_words(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter_map(|tag| match tag {
                CommandTag::Reply => Some(String::from("&REPLY")),
                CommandTag::Const => None,
                CommandTag::Super => Some(String::from("&SUPER")),
            })
            .chain(self.body.iter().map(|sec| match sec {
                CommandSection::Echo(txt) => String::from(txt),
                CommandSection::ChatterName => String::from("%name"),
                CommandSection::WordIndex(idx) => format!("%{idx}"),
            }))
            .collect()
    }

    pub fn is_const(&self) -> bool {
        self.tags.contains(&CommandTag::Const)
    }

    fn var_from_string(input: &str) -> Result<CommandSection, RulesError> {
        Ok(match input {
            "name" => CommandSection::ChatterName,
            input => {
                if let Ok(idx) = input.parse() {
                    CommandSection::WordIndex(idx)
                } else {
                    return Err(RulesError::BadVariable(String::from(input)));
                }
            }
        })
    }
    fn tag_from_string(input: &str) -> Result<CommandTag, RulesError> {
        Ok(match input {
            "REPLY" => CommandTag::Reply,
            "SUPER" => CommandTag::Super,
            input => return Err(RulesError::BadTag(String::from(input))),
        })
    }
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulesError::BadVariable(name) => {
                f.write_fmt(format_args!("Unknown variable {name:?}."))
            }
            RulesError::BadTag(name) => f.write_fmt(format_args!("Unknown tag {name:?}.")),
        }
    }
}
