use super::command::CommandRules;
use super::io;

pub fn register_base_commands(
    store: &super::Store,
    bot: &crate::bot::Bot,
) -> impl std::future::Future<Output = ()> + 'static {
    let data_mod = store.0.clone();
    let data_com = store.0.clone();
    let data_cus = store.0.clone();
    let commands: [std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>; 3] = [
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_mod.clone();
            async move {
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
                                    format!("Cannot set a builtin cmd {command_name:?}"),
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
                                    .insert(String::from(command_name), body);
                                tokio::spawn(io::refresh(data.clone()));
                            }
                            Err(err) => {
                                bot.reply(&msg, format!("Could not create command: {err}"))
                                    .await
                            }
                        }
                    } else {
                        bot.reply(
                            &msg,
                            String::from("Command \"cmd:set\" expects at least 2 arguments"),
                        )
                        .await
                    }
                } else if let Some(command_name) = msg.text.strip_prefix("!cmd:info") {
                    let command_name = command_name.trim();
                    if let Some(body) = data.read().await.commands.get(command_name) {
                        if body.is_builtin() {
                        } else {
                        }
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
                        bot.reply(&msg, format!("Unknown command {:?}", command_name))
                            .await;
                    }
                } else if let Some(command_name) = msg.text.strip_prefix("!cmd:remove") {
                    let command_name = command_name.trim();
                    if let Some(to_remove) = data.read().await.commands.get(command_name) {
                        if to_remove.is_builtin() {
                            bot.reply(
                                &msg,
                                format!("Cannot remove a builtin cmd {command_name:?}"),
                            )
                            .await;
                            return;
                        }
                        data.write().await.commands.remove(command_name);
                        tokio::spawn(io::refresh(data.clone()));
                    } else {
                        bot.reply(&msg, format!("Unknown command {:?}", command_name))
                            .await;
                    }
                } else if msg.text.starts_with("!shutdown") {
                    bot.shutdown().await;
                    return;
                }
            }
        })),
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_com.clone();
            async move {
                if msg.text.starts_with("!commands") {
                    bot.reply(
                        &msg,
                        format!(
                            "Commands: {}",
                            data.read()
                                .await
                                .commands
                                .iter()
                                .filter_map(|(k, v)| v.can_run(&msg, &bot).then_some(k))
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                    .await;
                }
            }
        })),
        Box::pin(bot.on_chat_message(move |msg, bot| {
            let data = data_cus.clone();
            async move {
                if let Some(command) = msg.text.strip_prefix('!') {
                    let words = command.trim().split(' ').collect::<Vec<_>>();
                    let [cmd, args @ ..] = words.as_slice() else { return; };
                    if let Some(command) = data.read().await.commands.get(*cmd) {
                        if !command.can_run(&msg, &bot) || command.is_builtin() {
                            return;
                        }

                        command
                            .execute(
                                args.into_iter().copied().map(String::from).collect(),
                                &msg,
                                &bot,
                            )
                            .await;
                    }
                }
            }
        })),
    ];

    let data = store.0.clone();
    async move {
        let mut data = data.write().await;
        for builtin in ["cmd:set", "commands", "cmd:info", "cmd:remove", "shutdown"] {
            data.commands.insert(
                String::from(builtin),
                super::command::CommandRules::empty_builtin(),
            );
        }
        drop(data);

        let mut set = tokio::task::JoinSet::new();
        for command in commands {
            set.spawn(command);
        }
        while let Some(Ok(())) = set.join_next().await {}
    }
}
