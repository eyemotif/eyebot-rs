use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default)]
pub struct Options {
    pub features: Features,
    pub exec: Exec,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Features {
    pub eye: bool,
    pub custom_commands: bool,
    pub counters: bool,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Exec {
    pub debug: bool,
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
            features: Features {
                eye: true,
                custom_commands: true,
                counters: true,
            },
            exec: Exec { debug: false },
        }
    }
}
