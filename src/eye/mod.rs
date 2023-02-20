use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

mod builtin;
mod command;
mod io;

#[derive(Debug)]
pub struct Store(StoreInner);

type StoreInner = Arc<RwLock<StoreData>>;

#[derive(Debug)]
struct StoreData {
    pub store_path: PathBuf,
    pub commands: HashMap<String, Arc<command::CommandRules>>,
    pub counters: HashMap<String, i64>,
    pub error_reporter: tokio::sync::mpsc::Sender<crate::bot::error::BotError>,
}

impl Store {
    pub async fn new<P: Into<PathBuf>>(
        store_path: P,
        bot: &crate::bot::Bot,
    ) -> std::io::Result<Self> {
        let store = Store(Arc::new(RwLock::new(StoreData {
            commands: HashMap::new(),
            counters: HashMap::new(),
            store_path: store_path.into(),
            error_reporter: bot.error_reporter(),
        })));
        io::load(store.0.clone()).await?;
        Ok(store)
    }
    pub fn register_base_commands(
        &self,
        bot: &crate::bot::Bot,
    ) -> impl std::future::Future<Output = ()> + 'static {
        builtin::register_base_commands(self, bot)
    }
}
