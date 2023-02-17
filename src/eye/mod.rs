use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
pub mod command;

#[derive(Debug)]
pub struct Store(Arc<Mutex<StoreData>>);

#[derive(Debug)]
struct StoreData {
    pub commands: HashMap<String, command::CommandRules>,
}

impl Store {
    pub fn new() -> Self {
        Store(Arc::new(Mutex::new(StoreData {
            commands: HashMap::from_iter(
                ["cmd:set", "commands", "cmd:info"]
                    .map(|k| (String::from(k), command::CommandRules::empty_const())),
            ),
        })))
    }
    pub fn register_base_commands(
        &self,
        bot: &crate::bot::Bot,
    ) -> impl std::future::Future<Output = ()> + 'static {
        let data_mod = self.0.clone();
        let data_com = data_mod.clone();
        let data_cus = data_mod.clone();
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
                                data.clone().lock().await.commands.get(command_name)
                            {
                                if existing_command.is_const() {
                                    bot.reply(
                                        &msg,
                                        format!("Cannot set a const cmd {command_name:?}"),
                                    )
                                    .await;
                                    return;
                                }
                            }
                            match command::CommandRules::parse(command_body) {
                                Ok(body) => {
                                    data.lock()
                                        .await
                                        .commands
                                        .insert(String::from(command_name), body);
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
                        if let Some(body) = data.lock().await.commands.get(command_name) {
                            let words = body.as_words();
                            bot.reply(
                                &msg,
                                format!(
                                    "!{}: {}",
                                    command_name,
                                    if body.is_const() {
                                        String::from("&CONST")
                                    } else {
                                        words.join(" ")
                                    }
                                ),
                            )
                            .await;
                        } else {
                            bot.reply(&msg, format!("Unknown command {:?}", command_name))
                                .await;
                        }
                    } else if let Some(command_name) = msg.text.strip_prefix("!cmd:remove") {
                        let command_name = command_name.trim();
                        let mut data = data.lock().await;
                        if let Some(to_remove) = data.commands.get(command_name) {
                            if to_remove.is_const() {
                                bot.reply(
                                    &msg,
                                    format!("Cannot remove a const cmd {command_name:?}"),
                                )
                                .await;
                                return;
                            }
                            data.commands.remove(command_name);
                        } else {
                            bot.reply(&msg, format!("Unknown command {:?}", command_name))
                                .await;
                        }
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
                                data.lock()
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
                        if let Some(command) = data.lock().await.commands.get(*cmd) {
                            if !command.can_run(&msg, &bot) {
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

        async move {
            let mut set = tokio::task::JoinSet::new();
            for command in commands {
                set.spawn(command);
            }
            while let Some(Ok(())) = set.join_next().await {}
        }
    }
}
