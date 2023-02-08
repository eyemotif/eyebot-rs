use auth::OAuthToken;
use clap::Parser;
use std::process::ExitCode;

pub mod auth;
mod chat;
mod cli;

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Cli::parse();

    let oauth = match args.oauth {
        Some(oauth) => OAuthToken(oauth),
        None => {
            println!("No OAuth provided. Starting server...");
            let auth = auth::oauth::OAuthServer::start_auth(auth::OAuthServerData {
                client_id: args.clientid.clone(),
                scopes: ["chat:read", "chat:edit"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                host_address: String::from("localhost:3000"),
                response_path: String::from("/response"),
            });

            let oauth = auth.into_inner().await.unwrap()?;
            println!("Success! Server closed.");
            oauth
        }
    };

    let token_manager = auth::access::AccessTokenManager::new(auth::AccessTokenManagerData {
        oauth,
        client_id: args.clientid.clone(),
        client_secret: args.clientsecret.clone(),
        redirect_url: String::from("http://localhost:3000"),
    })
    .await?;

    let chat_client = chat::client::ChatClient::new(chat::data::ChatClientData {
        access: token_manager,
        bot_username: String::from("eye___bot"),
        chat_channel: String::from("eye_motif"),
    })
    .await?;
    chat_client.handle_messages().await?;

    Ok(())
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
