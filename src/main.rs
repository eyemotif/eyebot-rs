use crate::eventsub::subscription::Subscription;
use auth::OAuthToken;
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

pub mod auth;
pub mod chat;
mod cli;
pub mod eventsub;
pub mod twitch;

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Cli::parse();
    let tokens_store_path = expand_store_path(args.store);

    let token_manager =
        match auth::access::AccessTokenManager::new_tokens(auth::AccessTokenManagerTokens {
            client_id: args.clientid.clone(),
            client_secret: args.clientsecret.clone(),
            redirect_url: String::from("http://localhost:3000"),
            tokens_store_path: tokens_store_path.clone(),
        })
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
            auth::access::AccessTokenManager::new_oauth(auth::AccessTokenManagerOAuth {
                oauth,
                client_id: args.clientid.clone(),
                client_secret: args.clientsecret.clone(),
                redirect_url: String::from("http://localhost:3000"),
                tokens_store_path,
            })
            .await?
        }
    };

    let chat_task = if args.chat {
        let chat_client = chat::client::ChatClient::new(chat::data::ChatClientData {
            access: token_manager.clone(),
            bot_username: String::from("eye___bot"),
            chat_channel: String::from("eye_motif"),
        })
        .await?;
        tokio::spawn(chat_client.on_chat(|message, bot| async move {
            if message.user_is_super() && message.text == "!ping" {
                bot.reply(&message, "Pong!");
            }
            if message.text.contains("egg") {
                bot.say("🥚");
            }
            if message.text == "frong" {
                bot.say("frong");
            }
            if !message.emotes.is_empty() {
                println!(
                    "{} -> {:?} {:?}",
                    message.strip_emotes(),
                    message.text,
                    message.emotes
                );
            }
        }));

        Some(chat_client.run())
    } else {
        None
    };

    let eventsub_task = if args.eventsub {
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

        let eventsub_client =
            eventsub::client::EventsubClient::new(eventsub::data::EventsubClientData {
                client_id: args.clientid.clone(),
                access: token_manager.clone(),
                subscriptions: vec![Subscription::ChannelPointRedeem {
                    broadcaster_user_id,
                    reward_id: None,
                }],
            })
            .await?;
        tokio::spawn(eventsub_client.on_message::<eventsub::data::NotificationMessage<eventsub::event::ChannelPointRedeem>, _> (|notification| async move {
            println!("User {} redeemed {}!", notification.payload.event.user_login, notification.payload.event.reward.title);
        }));
        Some(eventsub_client.run())
    } else {
        None
    };

    tokio::select! {
        Err(err) = async { println!("Running chat client!"); chat_task.unwrap().await }, if chat_task.is_some() => return Err(err.into()),
        Err(err) = async { println!("Running eventsub client!"); eventsub_task.unwrap().await }, if eventsub_task.is_some() => return Err(err.into()),
        else => (),
    }

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
            let auth = auth::oauth::OAuthServer::start_auth(auth::OAuthServerData {
                client_id,
                scopes: ["chat:read", "chat:edit", "channel:read:redemptions"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                host_address: String::from("localhost:3000"),
                response_path: String::from("/response"),
            });

            let oauth = auth.into_inner().await.unwrap()?;
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
