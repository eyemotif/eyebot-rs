pub mod auth;

#[tokio::main]
async fn main() {
    let auth = auth::oauth::OAuthClient::start_auth(auth::oauth::OAuthClientOptions {
        client_id: String::from(""),
        scopes: Vec::new(),
        host_address: String::from("localhost:3000"),
        response_path: String::from("/response"),
    });

    match auth.into_inner().await.unwrap() {
        Ok(token) => println!("got token! {:?}", token),
        Err(err) => eprintln!("{err:?}"),
    }
}
