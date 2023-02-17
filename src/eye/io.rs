use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;

pub(super) async fn refresh(data: Arc<RwLock<super::StoreData>>) -> std::io::Result<()> {
    let data = data.read().await;

    let commands_file = (
        data.store_path.join("commands.txt"),
        data.commands
            .iter()
            .filter_map(|(k, v)| (!v.is_const()).then_some(format!("{k} {}", v.as_words_string())))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    drop(data);
    tokio::try_join!(tokio::fs::write(commands_file.0, commands_file.1),)?;

    Ok(())
}

pub(super) async fn load(data: Arc<RwLock<super::StoreData>>) -> std::io::Result<()> {
    let mut data = data.write().await;

    for command in read_create(data.store_path.join("commands.txt"))
        .await?
        .split('\n')
    {
        if command.trim().is_empty() {
            continue;
        }

        if let Some((name, command)) = command.split_once(' ') {
            if data.commands.get(name).is_some() {
                continue;
            }

            if let Ok(command) = super::command::CommandRules::parse(command) {
                data.commands.insert(String::from(name), command);
            }
        }
    }
    Ok(())
}

async fn read_create<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .await?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).await?;
    Ok(buf)
}
