use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

mod builtin;
pub mod command;
mod io;

#[derive(Debug)]
pub struct Store(Arc<RwLock<StoreData>>);

#[derive(Debug)]
struct StoreData {
    pub store_path: PathBuf,
    pub commands: HashMap<String, command::CommandRules>,
}

impl Store {
    pub async fn new<P: Into<PathBuf>>(store_path: P) -> std::io::Result<Self> {
        let store = Store(Arc::new(RwLock::new(StoreData {
            commands: HashMap::new(),
            store_path: store_path.into(),
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
