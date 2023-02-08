use super::creds::OAuthToken;
use super::OAuthServerData;
use ring::rand::SecureRandom;
use std::collections::HashMap;
use std::error::Error;
use tiny_http::{Response, StatusCode};
use tokio::task::JoinHandle;

type ClientResult = std::result::Result<OAuthToken, OAuthServerError>;

#[derive(Debug)]
pub struct OAuthServer {
    join_handle: JoinHandle<ClientResult>,
}

#[derive(Debug)]
pub enum OAuthServerError {
    OnServerCreate(Box<dyn Error + Send + Sync>),
    OnReceive(std::io::Error),
    OnResponse(std::io::Error),
    OnAuth {
        error: String,
        error_description: String,
    },
    Ring(ring::error::Unspecified),
}

impl OAuthServer {
    pub fn start_auth(options: OAuthServerData) -> Self {
        let join_handle = tokio::spawn(OAuthServer::host_auth(options));
        OAuthServer { join_handle }
    }
    pub fn into_inner(self) -> JoinHandle<ClientResult> {
        self.join_handle
    }

    async fn host_auth(options: OAuthServerData) -> ClientResult {
        let server = tiny_http::Server::http(&options.host_address)
            .map_err(OAuthServerError::OnServerCreate)?;
        let rand = ring::rand::SystemRandom::new();
        let mut current_state = None;

        // https://docs.rs/ring/latest/ring/rand/struct.SystemRandom.html
        rand.fill(&mut []).map_err(OAuthServerError::Ring)?;

        loop {
            let request = server.recv().map_err(OAuthServerError::OnReceive)?;

            match request.url() {
                "/" => {
                    let (url, new_state) = OAuthServer::oauth_redirect_link(
                        &options.client_id,
                        &format!("http://{}{}", options.host_address, options.response_path),
                        &options.scopes,
                        &rand,
                    )
                    .map_err(OAuthServerError::Ring)?;

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
                    let Some(params) = OAuthServer::parse_url_params(response) else {
                        request.respond(OAuthServer::code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };

                    if let (Some(error), Some(error_description)) = (params.get("error"), params.get("error_description")) {
                        request.respond(OAuthServer::code(500, "Twitch error.")).map_err(OAuthServerError::OnResponse)?;
                        return Err(OAuthServerError::OnAuth { error: error.clone(), error_description: error_description.replace('+', " ") });
                    }

                    let (Some(code), Some(state)) = (params.get("code"), params.get("state")) else {
                        request.respond(OAuthServer::code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    let Some(current_state) = &current_state else {
                        request.respond(OAuthServer::code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    if current_state != state {
                        request.respond(OAuthServer::code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    }

                    request.respond(OAuthServer::code(200, "Success!")).map_err(OAuthServerError::OnResponse)?;
                    return Ok(OAuthToken(String::from(code)));
                }
                error if error.starts_with("/?error") => {
                    let Some(params) = OAuthServer::parse_url_params(&error[2..]) else {
                        request.respond(OAuthServer::code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };

                    let (Some(error), Some(error_description), Some(state)) = (params.get("error"), params.get("error_description"), params.get("state")) else {
                        request.respond(OAuthServer::code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    let Some(current_state) = &current_state else {
                        request.respond(OAuthServer::code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    if current_state != state {
                        request.respond(OAuthServer::code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    }

                    request.respond(OAuthServer::code(500, "Twitch error.")).map_err(OAuthServerError::OnResponse)?;
                    return Err(OAuthServerError::OnAuth {
                        error: String::from(error),
                        error_description: error_description.replace('+', " "),
                    });
                }
                _ => request.respond(OAuthServer::code(404, "Not found.")),
            }
            .map_err(OAuthServerError::OnResponse)?
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
        let state = buf.into_iter().map(|byte| format!("{byte:x?}")).collect();
        Ok((
            // cargo fmt doesn't format huge strings
            // TODO: force_verify option
            String::from("https://id.twitch.tv/oauth2/authorize?response_type=code&")
                + &format!(
                    "client_id={client_id}&redirect_uri={response_url}&state={state}&scope={}",
                    urlencoding::encode(&scopes.join(" "))
                ),
            state,
        ))
    }

    fn parse_url_params(params: &str) -> Option<HashMap<String, String>> {
        params
            .split('&')
            .map(|param| param.split_once('='))
            .map(|maybe_param| maybe_param.map(|(k, v)| (String::from(k), String::from(v))))
            .collect()
    }
}

impl std::fmt::Display for OAuthServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthServerError::OnServerCreate(err) => f.write_fmt(format_args!(
                "Error while creating the authentification server: {err}"
            )),
            OAuthServerError::OnReceive(err) => f.write_fmt(format_args!(
                "Error while trying to receive a request to the server: {err}"
            )),
            OAuthServerError::OnResponse(err) => f.write_fmt(format_args!(
                "Error while trying to send a response from the server: {err}"
            )),
            OAuthServerError::OnAuth {
                error,
                error_description,
            } => f.write_fmt(format_args!(
                "Error {error} while validating the user's credentials: {error_description}"
            )),
            OAuthServerError::Ring(err) => {
                f.write_fmt(format_args!("Error while creating random data: {err}"))
            }
        }
    }
}
impl std::error::Error for OAuthServerError {}
