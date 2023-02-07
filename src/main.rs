pub mod auth;

#[tokio::main]
async fn main() {
    let auth = auth::oauth::OAuthClient::start_auth(auth::oauth::OAuthClientData {
        client_id: String::from(""),
        scopes: Vec::new(),
        host_address: String::from("localhost:3000"),
        response_path: String::from("/response"),
    });

    let oauth = match auth.into_inner().await.unwrap() {
        Ok(token) => token,
        Err(err) => {
            eprintln!("Error while getting OAuth token: {err:?}");
            return;
        }
    };

    let token_manager =
        match auth::access::AccessTokenManager::new(auth::access::AccessTokenManagerData {
            oauth,
            client_id: String::from(""),
            client_secret: String::from(""),
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
