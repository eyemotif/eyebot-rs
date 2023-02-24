use super::comet;
use super::command::CommandRules;
use super::io;
use super::listener;
use regex::Regex;
use std::sync::Arc;

pub fn register_base_commands(
    store: &super::Store,
    bot: &crate::bot::Bot,
) -> impl std::future::Future<Output = ()> + 'static {
    let data_mod = store.0.clone();
    let data_cmd = store.0.clone();
    let data_cus = store.0.clone();
    let data_cnt = store.0.clone();
    let data_cmn = store.0.clone();
    let data_lis = store.0.clone();
    let data_lse = store.0.clone();

    let commands: [std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>; 7] = [
        // Mod-only commands
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let _data = data_mod.clone();
            async move {
                if !msg.user_is_super() {
                    return;
                }

                if msg.text.starts_with("!shutdown") {
                    bot.shutdown().await;
                    return;
                } else if msg.text.starts_with("!ping") {
                    bot.reply(&msg, "Pong!").await;
                }
            }
        })),
        // Custom command commands
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_cmd.clone();
            async move {
                if !data.read().await.options.features.custom_commands {
                    return;
                }
                if !msg.user_is_super() {
                    return;
                }

                if let Some(command) = msg.text.strip_prefix("!cmd:set") {
                    if let Some((command_name, command_body)) = command.trim().split_once(' ') {
                        if let Some(existing_command) =
                            data.clone().read().await.commands.get(command_name)
                        {
                            if existing_command.is_builtin() {
                                bot.reply(
                                    &msg,
                                    format!("Cannot set a builtin cmd {command_name:?}."),
                                )
                                .await;
                                return;
                            }
                        }
                        match CommandRules::parse(command_body) {
                            Ok(body) => {
                                data.write()
                                    .await
                                    .commands
                                    .insert(String::from(command_name), Arc::new(body));
                                io::spawn_io(data.clone(), io::refresh(data.clone()));
                            }
                            Err(err) => {
                                bot.reply(&msg, format!("Could not create command: {err}"))
                                    .await
                            }
                        }
                    } else {
                        bot.reply(
                            &msg,
                            String::from("Command \"cmd:set\" expects at least 2 arguments."),
                        )
                        .await
                    }
                } else if let Some(command_name) = msg.text.strip_prefix("!cmd:info") {
                    let command_name = command_name.trim();
                    if let Some(body) = data.read().await.commands.get(command_name) {
                        bot.reply(
                            &msg,
                            if body.is_builtin() {
                                format!("!{command_name} is a builtin command")
                            } else {
                                format!("!{command_name}: {}", body.as_words_string())
                            },
                        )
                        .await;
                    } else {
                        bot.reply(&msg, format!("Unknown command {command_name:?}."))
                            .await;
                    }
                } else if let Some(command_name) = msg.text.strip_prefix("!cmd:remove") {
                    let command_name = command_name.trim();
                    let mut data_write = data.write().await;
                    if let Some((command_name, to_remove)) =
                        data_write.commands.remove_entry(command_name)
                    {
                        if to_remove.is_builtin() {
                            bot.reply(
                                &msg,
                                format!("Cannot remove a builtin cmd {command_name:?}."),
                            )
                            .await;
                            data_write.commands.insert(command_name, to_remove);
                            return;
                        }

                        drop(data_write);
                        io::spawn_io(data.clone(), io::refresh(data.clone()));
                    } else {
                        bot.reply(&msg, format!("Unknown command {command_name:?}."))
                            .await;
                    }
                }
            }
        })),
        // Counter commands
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_cnt.clone();
            async move {
                if !data.read().await.options.features.counters {
                    return;
                }
                if !msg.user_is_super() {
                    return;
                }

                if let Some(args) = msg.text.strip_prefix("!counter:set") {
                    let args = args.trim();
                    if let Some((counter_name, counter_value)) = args.split_once(' ') {
                        if let Ok(value) = counter_value.parse() {
                            data.write()
                                .await
                                .counters
                                .insert(String::from(counter_name), value);
                            io::spawn_io(data.clone(), io::refresh(data.clone()));
                        } else {
                            bot.reply(&msg, format!("{counter_value:?} is not an integer."))
                                .await
                        }
                    } else {
                        bot.reply(
                            &msg,
                            String::from("Command \"counter:set\" expects at least 2 arguments."),
                        )
                        .await
                    }
                } else if let Some(counter_name) = msg.text.strip_prefix("!counter:remove") {
                    let counter_name = counter_name.trim();
                    if data.write().await.counters.remove(counter_name).is_some() {
                        io::spawn_io(data.clone(), io::refresh(data.clone()));
                    } else {
                        bot.reply(&msg, format!("Unknown counter {counter_name:?}."))
                            .await;
                    }
                } else if let Some(counter_name) = msg.text.strip_prefix("!counter:get") {
                    let counter_name = counter_name.trim();
                    if let Some(value) = data.read().await.counters.get(counter_name) {
                        bot.reply(&msg, format!("Counter {counter_name:?}: {value}"))
                            .await;
                    } else {
                        bot.reply(&msg, format!("Unknown counter {counter_name:?}."))
                            .await
                    }
                } else if msg.text.starts_with("!counter:list") {
                    let keys = data
                        .read()
                        .await
                        .counters
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>();

                    bot.reply(
                        &msg,
                        if keys.is_empty() {
                            String::from("No counters.")
                        } else {
                            format!("Counters: {}", keys.join(", "))
                        },
                    )
                    .await;
                }
            }
        })),
        // Listener commands
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_lis.clone();
            async move {
                if !data.read().await.options.features.listeners {
                    return;
                }
                if !msg.user_is_super() {
                    return;
                }

                if let Some(args) = msg.text.strip_prefix("!listen:exact") {
                    if let Some((name, pattern, command)) = listener::Listener::parts(args.trim()) {
                        match CommandRules::parse(&command) {
                            Ok(body) => {
                                data.write().await.listeners.insert(
                                    name,
                                    listener::Listener {
                                        predicate: listener::Predicate::Exactly(pattern),
                                        body,
                                    },
                                );
                                io::spawn_io(data.clone(), io::refresh(data.clone()));
                            }
                            Err(err) => {
                                bot.reply(&msg, format!("Could not create listener: {err}."))
                                    .await;
                            }
                        }
                    } else {
                        bot.reply(
                            &msg,
                            "Usage: !listen:exact listenname listenpatern/listencommand",
                        )
                        .await;
                    }
                } else if let Some(args) = msg.text.strip_prefix("!listen:has") {
                    if let Some((name, pattern, command)) = listener::Listener::parts(args.trim()) {
                        match CommandRules::parse(&command) {
                            Ok(body) => {
                                data.write().await.listeners.insert(
                                    name,
                                    listener::Listener {
                                        predicate: listener::Predicate::Contains(pattern),
                                        body,
                                    },
                                );
                                io::spawn_io(data.clone(), io::refresh(data.clone()));
                            }
                            Err(err) => {
                                bot.reply(&msg, format!("Could not create listener: {err}."))
                                    .await;
                            }
                        }
                    } else {
                        bot.reply(
                            &msg,
                            "Usage: !listen:has listenname listenpatern/listencommand",
                        )
                        .await;
                    }
                } else if let Some(args) = msg.text.strip_prefix("!listen:regex") {
                    if let Some((name, pattern, command)) = listener::Listener::parts(args.trim()) {
                        let regex = match Regex::new(&pattern) {
                            Ok(it) => it,
                            Err(_err) => {
                                // FIXME: Report regex errors
                                bot.reply(&msg, "Regex error.").await;
                                return;
                            }
                        };
                        match CommandRules::parse(&command) {
                            Ok(body) => {
                                data.write().await.listeners.insert(
                                    name,
                                    listener::Listener {
                                        predicate: listener::Predicate::Regex(regex),
                                        body,
                                    },
                                );
                                io::spawn_io(data.clone(), io::refresh(data.clone()));
                            }
                            Err(err) => {
                                bot.reply(&msg, format!("Could not create listener: {err}."))
                                    .await;
                            }
                        }
                    } else {
                        bot.reply(
                            &msg,
                            "Usage: !listen:regex listenname listenpatern/listencommand",
                        )
                        .await;
                    }
                } else if let Some(name) = msg.text.strip_prefix("!listen:info") {
                    let name = name.trim();
                    if let Some(listener) = data.read().await.listeners.get(name) {
                        bot.reply(
                            &msg,
                            format!(
                                "Listener {name} {}/{}",
                                match &listener.predicate {
                                    listener::Predicate::Exactly(pat) => format!("(exact): {pat}"),
                                    listener::Predicate::Contains(pat) =>
                                        format!("(contains): {pat}"),
                                    listener::Predicate::Regex(pat) => format!("(regex): {pat}"),
                                },
                                listener.body.as_words_string(),
                            ),
                        )
                        .await;
                    } else {
                        bot.reply(&msg, format!("Unknown listener {name}.")).await;
                    }
                } else if let Some(name) = msg.text.strip_prefix("!listen:remove") {
                    let name = name.trim();
                    if let Some(_) = data.write().await.listeners.remove(name) {
                        io::spawn_io(data.clone(), io::refresh(data.clone()));
                    } else {
                        bot.reply(&msg, format!("Unknown listener {name}.")).await;
                    }
                } else if msg.text.starts_with("!listen:list") {
                    let keys = data
                        .read()
                        .await
                        .listeners
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>();

                    bot.reply(
                        &msg,
                        if keys.is_empty() {
                            String::from("No listeners.")
                        } else {
                            format!("Listeners: {}", keys.join(", "))
                        },
                    )
                    .await;
                }
            }
        })),
        // Custom command executor
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_cus.clone();
            async move {
                if !data.read().await.options.features.custom_commands {
                    return;
                }

                if let Some(command) = msg.text.strip_prefix('!') {
                    let words = command.trim().split(' ').collect::<Vec<_>>();
                    let [cmd, args @ ..] = words.as_slice() else { return; };

                    let data_read = data.read().await;
                    if let Some(command) = data_read.commands.get(*cmd).cloned() {
                        if !command.can_run(&msg, &bot) || command.is_builtin() {
                            return;
                        }

                        drop(data_read);
                        command
                            .execute(
                                args.iter().copied().map(String::from).collect(),
                                &msg,
                                &bot,
                                data.clone(),
                            )
                            .await;
                    }
                }
            }
        })),
        // Listener executor
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_lse.clone();
            async move {
                if !data.read().await.options.features.listeners {
                    return;
                }

                for (_, listener) in &data.read().await.listeners {
                    listener.execute(&msg, &bot, data.clone()).await;
                }
            }
        })),
        // Common commands
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_cmn.clone();
            async move {
                if msg.text.starts_with("!commands") {
                    let mut commands = data
                        .read()
                        .await
                        .commands
                        .iter()
                        .filter_map(|(k, v)| v.can_run(&msg, &bot).then_some(k))
                        .cloned()
                        .collect::<Vec<_>>();
                    commands.sort_unstable();
                    bot.reply(&msg, format!("Commands: {}", commands.join(", ")))
                        .await;
                }
            }
        })),
    ];

    let data = store.0.clone();
    async move {
        let mut data = data.write().await;
        let mut builtins = vec![("shutdown", true), ("ping", true), ("commands", false)];

        if data.options.features.custom_commands {
            builtins.append(&mut vec![
                ("cmd:set", true),
                ("cmd:info", true),
                ("cmd:remove", true),
            ]);
        }
        if data.options.features.counters {
            builtins.append(&mut vec![
                ("counter:set", true),
                ("counter:get", true),
                ("counter:remove", true),
                ("counter:list", true),
            ]);
        }

        if data.options.features.listeners {
            builtins.append(&mut vec![
                ("listen:exact", true),
                ("listen:has", true),
                ("listen:regex", true),
                ("listen:list", true),
                ("listen:info", true),
                ("listen:remove", true),
            ]);
        }

        for (builtin, is_super) in builtins {
            data.commands.insert(
                String::from(builtin),
                Arc::new(super::command::CommandRules::empty_builtin(is_super)),
            );
        }
        drop(data);

        let mut set = tokio::task::JoinSet::new();
        for command in commands {
            set.spawn(command);
        }
        while let Some(join_result) = set.join_next().await {
            join_result.expect("Command task panicked");
        }
    }
}

pub fn register_comet_commands(
    store: &super::Store,
    bot: &crate::bot::Bot,
    comet_server: &comet::Server,
) -> impl std::future::Future<Output = ()> + 'static {
    let data = store.0.clone();
    let command_future = bot.on_chat_message_comet(comet_server, move |msg, bot, cmt| {
        let data = data.clone();
        async move {
            if let Some(arg) = msg.text.strip_prefix("!comet:get") {
                let component_type = match arg.trim() {
                    "audio" => comet::component::Type::Audio,
                    arg => {
                        bot.reply(&msg, format!("Unknown component type {arg:?}."))
                            .await;
                        return;
                    }
                };

                let Some(get_response) = cmt
                    .send_message(comet::Message::GetComponents { component_type })
                    .await else {
                        data.read().await.options.debug("Builtin: Comet client disconnected before response");
                        return; 
                    };

                match get_response {
                    comet::ResponseData::Ok => {
                        unreachable!("Expected data from GetComponents")
                    }
                    comet::ResponseData::Data { payload } => bot.reply(&msg, payload).await,
                    comet::ResponseData::Error {
                        is_internal,
                        message,
                    } => {
                        bot.reply(
                            &msg,
                            format!(
                                "{}Comet error: {message}",
                                if is_internal { "Internal " } else { "" }
                            ),
                        )
                        .await;
                    }
                }
            } else if let Some(body) = msg.text.strip_prefix("!comet:play-sound") {
                let sounds = body
                    .split([' ', ','])
                    .map(|word| {
                        word.split('+')
                            .map(|sound| comet::component::Sound {
                                name: String::from(sound),
                            })
                            .collect()
                    })
                    .collect();

                let Some(play_response) = cmt
                    .send_message(comet::Message::PlayAudio { data: sounds })
                    .await else {
                        data.read().await.options.debug("Builtin: Comet client disconnected before response");
                        return; 
                    };

                match play_response {
                    comet::ResponseData::Ok => (),
                    comet::ResponseData::Data { payload: _ } => unreachable!(),
                    comet::ResponseData::Error {
                        is_internal,
                        message,
                    } => {
                        bot.reply(
                            &msg,
                            format!(
                                "{}Comet error: {message}",
                                if is_internal { "Internal " } else { "" }
                            ),
                        )
                        .await;
                    }
                }
            } else if msg.text.starts_with("!comet:ping") {
                bot.reply(
                    &msg,
                    if cmt.has_client().await {
                        "Pong!"
                    } else {
                        "No Comet client."
                    },
                )
                .await;
            }
        }
    });

    let data = store.0.clone();
    async move {
        let mut data = data.write().await;
        for builtin in ["get", "play-sound"] {
            data.commands.insert(
                String::from(format!("comet:{builtin}")),
                Arc::new(super::command::CommandRules::empty_builtin(true)),
            );
        }
        drop(data);

        command_future.await;
    }
}
