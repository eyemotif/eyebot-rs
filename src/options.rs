use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub features: Features,
    pub exec: Exec,
    pub bot: Bot,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Features {
    pub eye: bool,
    pub custom_commands: bool,
    pub counters: bool,
    pub listeners: bool,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Exec {
    pub debug: bool,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Bot {
    pub duplicate_message_depth: usize,
}

impl Options {
    pub fn debug<S: Into<String>>(&self, s: S) {
        if self.exec.debug {
            println!("[DEBUG] {}", s.into());
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            features: Features::default(),
            exec: Exec::default(),
            bot: Bot::default(),
        }
    }
}
impl Default for Features {
    fn default() -> Self {
        Self {
            eye: true,
            custom_commands: true,
            counters: true,
            listeners: true,
        }
    }
}
impl Default for Exec {
    fn default() -> Self {
        Self { debug: false }
    }
}
impl Default for Bot {
    fn default() -> Self {
        Self {
            duplicate_message_depth: 0,
        }
    }
}
