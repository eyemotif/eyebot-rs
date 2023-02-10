use auth::OAuthToken;
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

pub mod auth;
pub mod chat;
mod cli;
pub mod eventsub;

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

    let chat_client = chat::client::ChatClient::new(chat::data::ChatClientData {
        access: token_manager,
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
    }));
    chat_client.run().await?;

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
                scopes: ["chat:read", "chat:edit"]
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
