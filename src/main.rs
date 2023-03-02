use crate::eventsub::subscription::Subscription;
use auth::OAuthToken;
use clap::Parser;
use eventsub::event;
use std::path::PathBuf;
use std::process::ExitCode;

pub mod auth;
pub mod bot;
pub mod chat;
mod cli;
pub mod eventsub;
pub mod eye;
pub mod options;
pub mod twitch;

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = cli::Cli::parse();
    let tokens_store_path = expand_store_path(args.store);
    let options = if let Some(options_file) = args.options_file {
        let path = PathBuf::from(&options_file);
        if !path.try_exists()? {
            return Err(format!("Options file {options_file:?} does not exist.").into());
        }
        toml::from_str::<options::Options>(&std::fs::read_to_string(path)?)?
    } else {
        options::Options::default()
    };

    let token_manager = match auth::access::AccessTokenManager::new_tokens(
        auth::AccessTokenManagerTokens {
            client_id: args.clientid.clone(),
            client_secret: args.clientsecret.clone(),
            redirect_url: String::from("http://localhost:3000"),
            tokens_store_path: tokens_store_path.join("access"),
        },
        options,
    )
    .await
    {
        Ok(_) if args.reauth => None,
        Ok(manager) => Some(manager),
        Err(auth::error::AccessTokenManagerError::InvalidTokens) => {
            println!("The stored tokens are invalid/missing!");
            None
        }
        Err(err) => return Err(err.into()),
    };

    let token_manager = match token_manager {
        Some(manager) => manager,
        None => {
            let oauth = run_oauth_server(args.oauth.clone(), args.clientid.clone()).await?;
            auth::access::AccessTokenManager::new_oauth(
                auth::AccessTokenManagerOAuth {
                    oauth,
                    client_id: args.clientid.clone(),
                    client_secret: args.clientsecret.clone(),
                    redirect_url: String::from("http://localhost:3000"),
                    tokens_store_path: tokens_store_path.join("access"),
                },
                options,
            )
            .await?
        }
    };

    let broadcaster_user_id = twitch::user_from_login(
        "eye_motif",
        &twitch::HelixAuth {
            client_id: args.clientid.clone(),
            access: token_manager.clone(),
        },
    )
    .await?
    .expect("Channel exists")
    .id;

    let bot = bot::Bot::new(
        bot::data::BotData {
            client_id: args.clientid,
            access: token_manager,
            bot_username: String::from("eye___bot"),
            chat_channel: String::from("eye_motif"),
            chat_implicit_access: args.chat_access,
            subscriptions: vec![
                Subscription::ChannelPointRedeem {
                    broadcaster_user_id: broadcaster_user_id.clone(),
                    reward_id: None,
                },
                Subscription::RaidTo {
                    broadcaster_user_id: broadcaster_user_id.clone(),
                },
            ],
        },
        options,
    )
    .await?;

    if options.features.eye {
        let eye_store = eye::Store::new(tokens_store_path.clone(), &bot, options).await?;
        tokio::spawn(eye_store.register_base_commands(&bot));

        tokio::spawn(bot.on_event::<event::Raid, _>(|notif, bot| async move {
            match twitch::stream_from_user_id(
                &notif.payload.event.from_broadcaster_user_id,
                bot.helix_auth(),
            )
            .await
            {
                Ok(Some(stream)) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    bot.say(format!(
                        "Thank you so much @{} for the raid!!! <3",
                        stream.user_name
                    ))
                    .await;
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    bot.say(format!(
                        "{} was last playing \"{}\" with {} viewer{}! :D",
                        stream.user_name,
                        stream.game_name,
                        stream.viewer_count,
                        if stream.viewer_count == 1 { "" } else { "s" }
                    ))
                    .await;
                }
                Ok(None) => bot.say("Thank you so much for the raid!!! <3").await,
                Err(err) => {
                    eprintln!("Error getting a Stream from a user id: {err}");
                    bot.say("Thank you so much for the raid!!! <3").await;
                }
            };
        }));

        if options.features.comet {
            // TODO: add options for port
            let comet_server =
                eye::comet::Server::new(8000, "eye_motif", bot.error_reporter(), options).await?;

            tokio::spawn(eye_store.register_comet_commands(&bot, &comet_server));

            tokio::spawn(bot.on_event_comet::<event::ChannelPointRedeem, _>(
                &comet_server,
                |notif, bot, cmt| async move {
                    if notif.payload.event.reward.title == "Play Audio" {
                        let input = notif
                            .payload
                            .event
                            .user_input
                            .expect("User input should be set");

                        let input = eye::comet::component::Sound::parse(&input);
                        match cmt
                            .send_message(eye::comet::Message::PlayAudio { data: input })
                            .await
                            .expect("Comet server should be open")
                        {
                            eye::comet::ResponseData::Ok => (),
                            eye::comet::ResponseData::Data { payload: _ } => unreachable!(),
                            eye::comet::ResponseData::Error {
                                is_internal,
                                message,
                            } => {
                                if !is_internal {
                                    bot.say(message).await;
                                }
                            }
                        }
                    }
                },
            ));

            tokio::spawn(
                bot.on_chat_message_comet(&comet_server, |msg, bot, cmt| async move {
                    match cmt.get_features().await {
                        Some(features)
                            if features.contains(&eye::comet::feature::Feature::Chat) =>
                        {
                            ()
                        }
                        _ => return,
                    }

                    let chat = eye::comet::component::Chat::from_chat_message(&msg);
                    loop {
                        match cmt
                            .send_message(eye::comet::Message::Chat {
                                user_id: msg.user_id.clone(),
                                chat: chat.clone(),
                            })
                            .await
                        {
                            Some(response) => match response {
                                eye::comet::ResponseData::Ok => break,
                                eye::comet::ResponseData::Data { payload } => {
                                    match cmt.send_message(eye::comet::Message::ChatUser {
                                        user_id: msg.user_id.clone(),
                                        chat_info: eye::comet::component::ChatterInfo {
                                            display_name: msg.display_name.clone(),

                                            name_color: msg.,

                                            badges: Vec < String,
                                        },
                                    }) {}
                                }
                                eye::comet::ResponseData::Error {
                                    is_internal,
                                    message,
                                } => {
                                    let _ = bot
                                        .error(
                                            format!(
                                                "{}Error sending a chat message to comet client: ",
                                                if is_internal { "Internal " } else { "" }
                                            ) + &message,
                                        )
                                        .await;
                                    return;
                                }
                            },
                            None => break,
                        }
                    }
                }),
            );

            tokio::spawn(comet_server.accept_connections());
        }
    }

    // tokio::spawn(
    //     bot.on_event::<event::ChannelPointRedeem, _>(|notif, bot| async move {
    //         if notif.payload.event.reward.title == "Pop" {
    //             bot.say(format!("{} redeemed Pop!", notif.payload.event.user_name))
    //                 .await;
    //         }
    //     }),
    // );

    bot.run().await?;

    Ok(())
}

async fn run_oauth_server(
    oauth: Option<String>,
    client_id: String,
) -> Result<OAuthToken, auth::error::OAuthServerError> {
    match oauth {
        Some(oauth) => Ok(OAuthToken(oauth)),
        None => {
            println!("No OAuth provided. Starting server at http:://localhost:3000 ...");
            let oauth = auth::oauth::oauth_server(auth::OAuthServerData {
                client_id,
                scopes: [
                    "chat:read",
                    "chat:edit",
                    "channel:read:redemptions",
                    "channel:read:subscriptions",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                host_address: String::from("localhost:3000"),
                response_path: String::from("/response"),
            })
            .await?;

            println!("Success! Server closed.");
            Ok(oauth)
        }
    }
}

fn expand_store_path(path: Option<String>) -> PathBuf {
    let path = path.unwrap_or(String::from("~/.eyebot-store"));
    if let Some(path) = path.strip_prefix("~/") {
        let mut home = home::home_dir().expect("No home directory found");
        home.push(path);
        home
    } else {
        PathBuf::from(path)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
