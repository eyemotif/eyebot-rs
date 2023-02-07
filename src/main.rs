use auth::creds::OAuthToken;
use clap::Parser;

pub mod auth;
mod cli;

#[tokio::main]
async fn main() {
    let args = cli::Cli::parse();

    let oauth = match args.oauth {
        Some(oauth) => OAuthToken(oauth),
        None => {
            println!("No OAuth provided, starting server...");
            let auth = auth::oauth::OAuthClient::start_auth(auth::oauth::OAuthClientData {
                client_id: args.clientid.clone(),
                scopes: Vec::new(), // TODO: add scopes
                host_address: String::from("localhost:3000"),
                response_path: String::from("/response"),
            });

            match auth.into_inner().await.unwrap() {
                Ok(token) => {
                    println!("Server closed.\nGot OAuth token: {}", token.0);
                    token
                }
                Err(err) => {
                    eprintln!("Error while getting OAuth token: {err:?}");
                    return;
                }
            }
        }
    };

    let token_manager =
        match auth::access::AccessTokenManager::new(auth::access::AccessTokenManagerData {
            oauth,
            client_id: args.clientid.clone(),
            client_secret: args.clientsecret.clone(),
            redirect_url: String::from("http://localhost:3000"),
        })
        .await
        {
            Ok(manager) => manager,
            Err(err) => {
                eprintln!("Error while getting an access token: {err:?}");
                return;
            }
        };

    println!(
        "Access granted: {:?}",
        token_manager.get_credentials().await
    )
}
