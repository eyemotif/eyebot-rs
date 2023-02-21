use crate::bot::interface::BotInterface;
use crate::chat::data::ChatMessage;
use regex::Regex;

#[derive(Debug)]
pub struct Listener {
    pub predicate: Predicate,
    pub body: super::command::CommandRules,
}

#[derive(Debug)]
pub enum Predicate {
    Exactly(String),
    Contains(String),
    Regex(Regex),
}

impl Listener {
    pub(super) async fn execute(
        &self,
        msg: &ChatMessage,
        bot: &BotInterface,
        data: super::StoreInner,
    ) {
        let message = msg.text.trim();
        let Some(args) = self.predicate.args(message) else { return; };

        self.body.execute(args, msg, bot, data).await;
    }

    pub(super) fn parts(args: &str) -> Option<(String, String, String)> {
        lazy_static::lazy_static! {
            static ref CAPTURE: Regex = Regex::new(r"(.+) ((?:[^/\\]|\\.)+)/(.+)").expect("Static regex");
            static ref UNESCAPE: Regex = Regex::new(r"\\(.)").expect("Static regex");
        }

        let captures = CAPTURE.captures(args)?;

        let name = String::from(captures.get(1)?.as_str());
        let pattern = UNESCAPE
            .replace(captures.get(2)?.as_str(), "$1")
            .into_owned();
        let command = String::from(captures.get(3)?.as_str());

        Some((name, pattern, command))
    }
}

impl Predicate {
    fn args(&self, message: &str) -> Option<Vec<String>> {
        match self {
            Predicate::Exactly(pat) => (message == pat).then_some(Vec::new()),
            Predicate::Contains(pat) => message.contains(pat).then_some(Vec::new()),
            Predicate::Regex(pat) => pat.captures(message).map(|cap| {
                cap.iter()
                    .filter_map(|mch| mch.map(|mch| String::from(mch.as_str())))
                    .collect()
            }),
        }
    }
}
