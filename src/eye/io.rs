use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::RwLock;

pub(super) fn spawn_io(
    data: super::StoreInner,
    fut: impl Future<Output = std::io::Result<()>> + Send + 'static,
) {
    tokio::spawn(async move {
        match fut.await {
            Ok(()) => (),
            Err(err) => {
                let _ = data
                    .read()
                    .await
                    .error_reporter
                    .send(crate::bot::error::BotError::IO(err))
                    .await;
            }
        }
    });
}

pub(super) async fn refresh(data: super::StoreInner) -> std::io::Result<()> {
    let data = data.read().await;

    let mut stores = Vec::new();

    if data.options.features.custom_commands {
        stores.push((
            data.store_path.join("commands.txt"),
            data.commands
                .iter()
                .filter_map(|(k, v)| {
                    (!v.is_builtin() && !v.is_temporary())
                        .then_some(format!("{k} {}", v.as_words_string()))
                })
                .collect::<Vec<_>>()
                .join("\n"),
        ))
    }
    if data.options.features.counters {
        stores.push((
            data.store_path.join("counters.txt"),
            data.counters
                .iter()
                .map(|(k, v)| format!("{k} {v}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }

    drop(data);

    let mut set = tokio::task::JoinSet::new();
    for (path, store) in stores {
        set.spawn(tokio::fs::write(path, store));
    }
    while let Some(join_result) = set.join_next().await {
        join_result.expect("io::refresh panicked")?;
    }

    Ok(())
}

pub(super) async fn load(data: Arc<RwLock<super::StoreData>>) -> std::io::Result<()> {
    let mut data = data.write().await;

    if data.options.features.custom_commands {
        for command in read_create(data.store_path.join("commands.txt")).await? {
            let command = command.trim();

            if let Some((name, command)) = command.split_once(' ') {
                if data.commands.get(name).is_some() {
                    continue;
                }

                if let Ok(command) = super::command::CommandRules::parse(command) {
                    data.commands.insert(String::from(name), Arc::new(command));
                }
            }
        }
    }

    if data.options.features.counters {
        for counter in read_create(data.store_path.join("counters.txt")).await? {
            let counter = counter.trim();

            if let Some((name, count)) = counter.split_once(' ') {
                if let Ok(count) = count.parse() {
                    data.counters.insert(String::from(name), count);
                }
            }
        }
    }

    Ok(())
}

async fn read_create<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .await?;
    let file = tokio::io::BufReader::new(file);
    let mut file = file.lines();

    let mut buf = Vec::new();
    while let Some(line) = file.next_line().await? {
        buf.push(line);
    }

    Ok(buf)
}
