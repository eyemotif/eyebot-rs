use super::io;
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
    Counter(String),
}
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum CommandTag {
    Reply,
    Builtin,
    Super,
    Temporary,
    CountInc(String),
    CountDec(String),
    CountReset(String),
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
                            } + " ",
                        ));
                        current_word = String::new();
                    }
                }
                '%' if !escape => {
                    if !(current_word.starts_with('%') || current_word.starts_with('&')) {
                        output.body.push(CommandSection::Echo(current_word + ""));
                        current_word = String::from("%");
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
                'a'..='z' | 'A'..='Z' | '0'..='9' | '=' | '_' => current_word.push(chr),
                misc_chr => {
                    if let Some(var_string) = current_word.strip_prefix('%') {
                        output.body.push(CommandRules::var_from_string(var_string)?);
                        current_word.clear();
                        current_word.push(misc_chr)
                    } else {
                        current_word.push(misc_chr)
                    }
                }
            }
            escape = false;
        }

        Ok(output)
    }

    #[must_use]
    pub fn empty_builtin(is_super: bool) -> Self {
        Self {
            body: Vec::new(),
            tags: HashSet::from_iter(if is_super {
                vec![CommandTag::Builtin, CommandTag::Super]
            } else {
                vec![CommandTag::Builtin]
            }),
        }
    }

    pub(super) async fn execute(
        &self,
        args: Vec<String>,
        msg: &ChatMessage,
        bot: &BotInterface,
        data: super::StoreInner,
    ) {
        enum OutputType {
            Normal,
            Reply,
        }
        let mut output_type = OutputType::Normal;

        for tag in &self.tags {
            // purposefully omitted a _ case to get get errors on adding a new CommandTag
            match tag {
                CommandTag::Reply => output_type = OutputType::Reply,
                CommandTag::Builtin => (),
                CommandTag::Super => (),
                CommandTag::Temporary => (),
                CommandTag::CountInc(name) => {
                    if let Some(counter_value) = data.write().await.counters.get_mut(name) {
                        *counter_value += 1;
                        io::spawn_io(data.clone(), io::refresh(data.clone()));
                    } else {
                        bot.reply(&msg, format!("Error: Counter {name:?} not found."))
                            .await;
                        return;
                    }
                }
                CommandTag::CountDec(name) => {
                    if let Some(counter_value) = data.write().await.counters.get_mut(name) {
                        *counter_value -= 1;
                        io::spawn_io(data.clone(), io::refresh(data.clone()));
                    } else {
                        bot.reply(&msg, format!("Error: Counter {name:?} not found."))
                            .await;
                        return;
                    }
                }
                CommandTag::CountReset(name) => {
                    if let Some(counter_value) = data.write().await.counters.get_mut(name) {
                        *counter_value = 0;
                        io::spawn_io(data.clone(), io::refresh(data.clone()));
                    } else {
                        bot.reply(&msg, format!("Error: Counter {name:?} not found."))
                            .await;
                        return;
                    }
                }
            }
        }

        let mut chatter_name_cache: Option<String> = None;
        let mut message = Vec::new();

        for section in &self.body {
            message.push(match section {
                CommandSection::Echo(text) => String::from(text),
                CommandSection::ChatterName => {
                    if let Some(chatter_name) = &chatter_name_cache {
                        chatter_name.clone()
                    } else {
                        let Ok(Some(user)) = crate::twitch::user_from_id(&msg.user_id, bot.helix_auth()).await else {
                            bot.error(format!("Could not get username from id {:?}", msg.user_id)).await;
                            return;
                        };
                        chatter_name_cache = Some(user.display_name.clone());
                        user.display_name
                    }
                }
                CommandSection::WordIndex(index) => {
                    String::from(args.get(*index).unwrap_or(&index.to_string()))
                },
                CommandSection::Counter(name) => if let Some(counter_value) = data.read().await.counters.get(name) {
                    counter_value.to_string()
                } else {
                    bot.reply(&msg, format!("Error: Counter {name:?} not found.")).await;
                    return;
                },
            })
        }

        let message = message.join("");

        match output_type {
            OutputType::Normal => bot.say(message).await,
            OutputType::Reply => bot.reply(&msg, message).await,
        }
    }

    #[must_use]
    pub fn can_run(&self, msg: &ChatMessage, _bot: &BotInterface) -> bool {
        if self.tags.contains(&CommandTag::Super) && !msg.user_is_super() {
            return false;
        }
        true
    }

    #[must_use]
    pub fn as_words_string(&self) -> String {
        self.tags
            .iter()
            .filter_map(|tag| match tag {
                CommandTag::Reply => Some(String::from("&REPLY")),
                CommandTag::Builtin => None,
                CommandTag::Super => Some(String::from("&SUPER")),
                CommandTag::Temporary => Some(String::from("&TEMP")),
                CommandTag::CountInc(name) => Some(format!("&C:INC={name}")),
                CommandTag::CountDec(name) => Some(format!("&C:DEC={name}")),
                CommandTag::CountReset(name) => Some(format!("&C:ZERO={name}")),
            })
            .map(|tag| tag + " ")
            .chain(self.body.iter().map(|sec| match sec {
                CommandSection::Echo(txt) => String::from(txt),
                CommandSection::ChatterName => String::from("%name"),
                CommandSection::WordIndex(idx) => format!("%{idx}"),
                CommandSection::Counter(name) => format!("%counter={name}"),
            }))
            .collect()
    }

    #[must_use]
    pub fn is_builtin(&self) -> bool {
        self.tags.contains(&CommandTag::Builtin)
    }
    #[must_use]
    pub fn is_temporary(&self) -> bool {
        self.tags.contains(&CommandTag::Temporary)
    }

    fn var_from_string(input: &str) -> Result<CommandSection, RulesError> {
        Ok(match input {
            "name" => CommandSection::ChatterName,
            input => {
                if let Ok(idx) = input.parse() {
                    CommandSection::WordIndex(idx)
                } else if let Some(counter_name) = input.strip_prefix("counter=") {
                    CommandSection::Counter(String::from(counter_name))
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
            "TEMP" => CommandTag::Temporary,
            input => {
                if let Some((tag, val)) = input.split_once('=') {
                    let val = String::from(val);
                    match tag {
                        "C:INC" => CommandTag::CountInc(val),
                        "C:DEC" => CommandTag::CountDec(val),
                        "C:ZERO" => CommandTag::CountReset(val),
                        input => return Err(RulesError::BadTag(String::from(input))),
                    }
                } else {
                    return Err(RulesError::BadTag(String::from(input)));
                }
            }
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
