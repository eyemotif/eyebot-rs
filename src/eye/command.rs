use crate::bot::interface::BotInterface;
use crate::chat::data::ChatMessage;
use std::future::Future;

pub struct Command<F1: Future<Output = bool>, F2: Future> {
    pub info: CommandInfo,
    pub can_run: Box<dyn Fn(&ChatMessage) -> F1>,
    pub run: Box<dyn Fn(Vec<String>, ChatMessage, BotInterface) -> F2>,
}

#[derive(Debug)]
pub struct CommandInfo {
    pub name: String,
    pub args: ArgsLength,
    pub usage: String,
}
#[derive(Debug)]
pub enum ArgsLength {
    Exactly(usize),
    AtLeast(usize),
    AtMost(usize),
}

#[derive(Debug)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
}

impl<F1: Future<Output = bool>, F2: Future> Command<F1, F2> {
    pub fn new(
        info: CommandInfo,
        can_run: impl Fn(&ChatMessage) -> F1 + 'static,
        run: impl Fn(Vec<String>, ChatMessage, BotInterface) -> F2 + 'static,
    ) -> Self {
        Self {
            info,
            can_run: Box::new(can_run),
            run: Box::new(run),
        }
    }
    pub async fn try_run(
        &self,
        chat_message: ChatMessage,
        bot_interface: BotInterface,
        parsed: ParsedCommand,
    ) -> bool {
        if !(self.can_run)(&chat_message).await {
            return false;
        }
        if !self.info.matches_shape(&parsed) {
            return false;
        }

        (self.run)(parsed.args, chat_message, bot_interface).await;

        true
    }
}

impl CommandInfo {
    pub fn matches_shape(&self, parsed_command: &ParsedCommand) -> bool {
        if self.name != parsed_command.name {
            return false;
        }
        match self.args {
            ArgsLength::Exactly(len) => parsed_command.args.len() == len,
            ArgsLength::AtLeast(len) => parsed_command.args.len() >= len,
            ArgsLength::AtMost(len) => parsed_command.args.len() <= len,
        }
    }
}

impl ParsedCommand {
    pub fn parse(input: &str) -> Self {
        #[derive(PartialEq, Eq)]
        enum ArgState {
            Normal(String),
            String(String),
        }

        let mut escape = false;
        let mut name = String::new();
        let mut args = Vec::new();

        for chr in input.chars() {
            match chr {
                ' ' => match args.last_mut() {
                    Some(ArgState::Normal(_)) | None => args.push(ArgState::Normal(String::new())),
                    Some(ArgState::String(word)) => *word += " ",
                },
                '"' => match args.last_mut() {
                    Some(ArgState::Normal(word)) | Some(ArgState::String(word)) if escape => {
                        *word += "\"";
                    }
                    None if escape => name += "\"",
                    Some(word) if *word == ArgState::Normal(String::new()) => {
                        *word = ArgState::String(String::new());
                    }
                    Some(ArgState::Normal(_)) | None => args.push(ArgState::String(String::new())),
                    Some(ArgState::String(_)) => args.push(ArgState::Normal(String::new())),
                },
                '\\' => {
                    if escape {
                        match args.last_mut() {
                            Some(ArgState::Normal(word)) | Some(ArgState::String(word)) => {
                                *word += "\\"
                            }
                            None => name += "\\",
                        }
                    } else {
                        escape = true;
                        continue;
                    }
                }
                chr => match args.last_mut() {
                    Some(ArgState::Normal(word)) | Some(ArgState::String(word)) => {
                        *word += &String::from(chr)
                    }
                    None => name += &String::from(chr),
                },
            }
            escape = false;
        }

        ParsedCommand {
            name,
            args: args
                .into_iter()
                .map(|arg| match arg {
                    ArgState::Normal(arg) | ArgState::String(arg) => arg,
                })
                .collect(),
        }
    }
}
