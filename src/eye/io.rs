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
        data.options.debug("Eye: Writing custom commands");

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
        data.options.debug("Eye: Writing counters");

        stores.push((
            data.store_path.join("counters.txt"),
            data.counters
                .iter()
                .map(|(k, v)| format!("{k} {v}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    if data.options.features.listeners {
        data.options.debug("Eye: Writing listeners");

        stores.push((
            data.store_path.join("listeners.txt"),
            data.listeners
                .iter()
                .map(|(k, v)| {
                    let (kind, pattern) = match &v.predicate {
                        super::listener::Predicate::Exactly(pat) => ('e', pat.as_str()),
                        super::listener::Predicate::Contains(pat) => ('c', pat.as_str()),
                        super::listener::Predicate::Regex(pat) => ('r', pat.as_str()),
                    };
                    format!(
                        "{kind} {k} {}/{}",
                        pattern.replace('\\', "\\\\").replace('/', "\\/"),
                        v.body.as_words_string()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        ))
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
        data.options.debug("Eye: Loading custom commands");

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
        data.options.debug("Eye: Loading counters");

        for counter in read_create(data.store_path.join("counters.txt")).await? {
            let counter = counter.trim();

            if let Some((name, count)) = counter.split_once(' ') {
                if let Ok(count) = count.parse() {
                    data.counters.insert(String::from(name), count);
                }
            }
        }
    }

    if data.options.features.listeners {
        data.options.debug("Eye: Loading listeners");

        for listener in read_create(data.store_path.join("listeners.txt")).await? {
            let listener = listener.trim();

            let kind = listener.chars().next().unwrap();
            let Some((name, pattern, command)) = super::listener::Listener::parts(&listener.chars().skip(2).collect::<String>()) else { continue; };

            let predicate = match kind {
                'e' => super::listener::Predicate::Exactly(pattern),
                'c' => super::listener::Predicate::Contains(pattern),
                'r' => {
                    let Ok(regex) = regex::Regex::new(&pattern) else { continue; };
                    super::listener::Predicate::Regex(regex)
                }
                _ => continue,
            };

            let Ok(body) = super::command::CommandRules::parse(&command) else { continue; };

            data.listeners
                .insert(name, super::listener::Listener { predicate, body });
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
