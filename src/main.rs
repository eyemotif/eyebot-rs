use std::process::ExitCode;

use auth::creds::OAuthToken;
use clap::Parser;

pub mod auth;
mod cli;

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Cli::parse();

    let oauth = match args.oauth {
        Some(oauth) => OAuthToken(oauth),
        None => {
            println!("No OAuth provided. Starting server...");
            let auth = auth::oauth::OAuthClient::start_auth(auth::oauth::OAuthClientData {
                client_id: args.clientid.clone(),
                scopes: Vec::new(), // TODO: add scopes
                host_address: String::from("localhost:3000"),
                response_path: String::from("/response"),
            });

            let oauth = auth.into_inner().await.unwrap()?;
            println!("Success! Server closed.");
            oauth
        }
    };

    let token_manager =
        auth::access::AccessTokenManager::new(auth::access::AccessTokenManagerData {
            oauth,
            client_id: args.clientid.clone(),
            client_secret: args.clientsecret.clone(),
            redirect_url: String::from("http://localhost:3000"),
        })
        .await?;

    println!(
        "Access granted: {:?}",
        token_manager.get_credentials().await
    );

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
