use super::creds::OAuthToken;
use ring::rand::SecureRandom;
use std::collections::HashMap;
use std::error::Error;
use tiny_http::{Response, StatusCode};
use tokio::task::JoinHandle;

type ClientJoinHandle = JoinHandle<Result<OAuthToken, OAuthClientError>>;

#[derive(Debug)]
pub struct OAuthClient {
    join_handle: ClientJoinHandle,
}

#[derive(Debug)]
pub struct OAuthClientOptions {
    pub client_id: String,
    pub host_address: String,
    pub response_path: String,
    pub scopes: Vec<String>,
}

#[derive(Debug)]
pub enum OAuthClientError {
    ServerError(Box<dyn Error + Send + Sync>),
    ReceiveError(std::io::Error),
    RespondError(std::io::Error),
    AuthError {
        error: String,
        error_description: String,
    },
    RandError(ring::error::Unspecified),
}

impl OAuthClient {
    pub fn start_auth(options: OAuthClientOptions) -> Self {
        let join_handle = tokio::spawn(OAuthClient::host_auth(options));
        OAuthClient { join_handle }
    }
    pub fn into_inner(self) -> ClientJoinHandle {
        self.join_handle
    }

    async fn host_auth(options: OAuthClientOptions) -> Result<OAuthToken, OAuthClientError> {
        let server = tiny_http::Server::http(&options.host_address)
            .map_err(OAuthClientError::ServerError)?;
        let rand = ring::rand::SystemRandom::new();
        let mut current_state = None;

        // https://docs.rs/ring/latest/ring/rand/struct.SystemRandom.html
        rand.fill(&mut []).map_err(OAuthClientError::RandError)?;

        loop {
            let request = server.recv().map_err(OAuthClientError::ReceiveError)?;

            match request.url() {
                "/" => {
                    let (url, new_state) = OAuthClient::oauth_redirect_link(
                        &options.client_id,
                        &format!("http://{}{}", options.host_address, options.response_path),
                        &options.scopes,
                        &rand,
                    )
                    .map_err(OAuthClientError::RandError)?;

                    current_state = Some(new_state);

                    request.respond(Response::new(
                        StatusCode(308),
                        vec![tiny_http::Header::from_bytes("Location".as_bytes(), url).unwrap()],
                        "Redirecting...".as_bytes(),
                        None,
                        None,
                    ))
                }
                response if response.starts_with(&options.response_path) => {
                    let (_, response) = response.split_once('?').unwrap();
                    let Some(params) = OAuthClient::parse_url_params(response) else {
                        request.respond(OAuthClient::code(400, "Invalid response.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    };

                    let (Some(code), Some(state)) = (params.get("code"), params.get("state")) else {
                        request.respond(OAuthClient::code(400, "Invalid response.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    };
                    let Some(current_state) = &current_state else {
                        request.respond(OAuthClient::code(403, "Invalid state.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    };
                    if current_state != state {
                        request.respond(OAuthClient::code(403, "Invalid state.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    }

                    request.respond(OAuthClient::code(200, "Success!")).map_err(OAuthClientError::RespondError)?;
                    return Ok(OAuthToken(String::from(code)));
                }
                error if error.starts_with("/?error") => {
                    let Some(params) = OAuthClient::parse_url_params(&error[2..]) else {
                        request.respond(OAuthClient::code(400, "Invalid response.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    };

                    let (Some(error), Some(error_description), Some(state)) = (params.get("error"), params.get("error_description"), params.get("state")) else {
                        request.respond(OAuthClient::code(400, "Invalid response.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    };
                    let Some(current_state) = &current_state else {
                        request.respond(OAuthClient::code(403, "Invalid state.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    };
                    if current_state != state {
                        request.respond(OAuthClient::code(403, "Invalid state.")).map_err(OAuthClientError::RespondError)?;
                        continue;
                    }

                    request.respond(OAuthClient::code(500, "Twitch error.")).map_err(OAuthClientError::RespondError)?;
                    return Err(OAuthClientError::AuthError {
                        error: String::from(error),
                        error_description: error_description.replace('+', " "),
                    });
                }
                _ => request.respond(OAuthClient::code(404, "Not found.")),
            }
            .map_err(OAuthClientError::RespondError)?
        }
    }

    fn code(code: u16, description: &str) -> Response<&[u8]> {
        Response::new(
            StatusCode(code),
            vec![
                tiny_http::Header::from_bytes("Content-Type".as_bytes(), "text/plain".as_bytes())
                    .unwrap(),
            ],
            description.as_bytes(),
            Some(description.len()),
            None,
        )
    }

    fn oauth_redirect_link(
        client_id: &str,
        response_url: &str,
        scopes: &[String],
        rng: &ring::rand::SystemRandom,
    ) -> Result<(String, String), ring::error::Unspecified> {
        let mut buf = [0; 32];
        rng.fill(&mut buf)?;
        let state = buf.into_iter().map(|byte| format!("{:x?}", byte)).collect();
        Ok((format!("https://id.twitch.tv/oauth2/authorize?response_type=code&force_verify=true&client_id={client_id}&redirect_uri={response_url}&state={state}&scope={}", urlencoding::encode(&scopes.join(" "))), state))
    }

    fn parse_url_params(params: &str) -> Option<HashMap<String, String>> {
        params
            .split('&')
            .map(|param| param.split_once('='))
            .map(|maybe_param| maybe_param.map(|(k, v)| (String::from(k), String::from(v))))
            .collect()
    }
}
