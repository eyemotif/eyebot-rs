//! An HTTP server that creates User Access tokens.
use super::creds::OAuthToken;
use super::error::OAuthServerError;
use super::OAuthServerData;
use ring::rand::SecureRandom;
use std::collections::HashMap;
use tiny_http::{Response, StatusCode};

type ClientResult = std::result::Result<OAuthToken, OAuthServerError>;

/// Runs an HTTP server to request a new Twitch OAuth token from the user.
///
/// # Errors
/// Returns an [OAuthServerError] if the server failed to obtain an OAuth token.
pub async fn oauth_server(options: OAuthServerData) -> ClientResult {
    let server =
        tiny_http::Server::http(&options.host_address).map_err(OAuthServerError::OnServerCreate)?;
    let rand = ring::rand::SystemRandom::new();
    let mut current_state = None;

    // https://docs.rs/ring/latest/ring/rand/struct.SystemRandom.html
    rand.fill(&mut []).map_err(OAuthServerError::Ring)?;

    loop {
        let request = server.recv().map_err(OAuthServerError::OnReceive)?;

        match request.url() {
                "/" => {
                    let (url, new_state) = oauth_redirect_link(
                        &options.client_id,
                        &format!("http://{}{}", options.host_address, options.response_path),
                        &options.scopes,
                        &rand,
                    )
                    .map_err(OAuthServerError::Ring)?;

                    current_state = Some(new_state);

                    request.respond(Response::new(
                        StatusCode(308),
                        vec![tiny_http::Header::from_bytes("Location", url).unwrap(), tiny_http::Header::from_bytes("Cache-Control", "no-store").unwrap()],
                        "Redirecting...".as_bytes(),
                        None,
                        None,
                    ))
                }
                response if response.starts_with(&options.response_path) => {
                    let (_, response) = response.split_once('?').unwrap();
                    let Some(params) = parse_url_params(response) else {
                        request.respond(respond_code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };

                    if let (Some(error), Some(error_description)) = (params.get("error"), params.get("error_description")) {
                        request.respond(respond_code(500, "Twitch error.")).map_err(OAuthServerError::OnResponse)?;
                        return Err(OAuthServerError::OnAuth { error: error.clone(), error_description: error_description.replace('+', " ") });
                    }

                    let (Some(code), Some(state)) = (params.get("code"), params.get("state")) else {
                        request.respond(respond_code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    let Some(current_state) = &current_state else {
                        request.respond(respond_code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    if current_state != state {
                        request.respond(respond_code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    }

                    request.respond(respond_code(200, "Success!")).map_err(OAuthServerError::OnResponse)?;
                    return Ok(OAuthToken(String::from(code)));
                }
                error if error.starts_with("/?error") => {
                    let Some(params) = parse_url_params(&error[2..]) else {
                        request.respond(respond_code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };

                    let (Some(error), Some(error_description), Some(state)) = (params.get("error"), params.get("error_description"), params.get("state")) else {
                        request.respond(respond_code(400, "Invalid response.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    let Some(current_state) = &current_state else {
                        request.respond(respond_code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    };
                    if current_state != state {
                        request.respond(respond_code(403, "Invalid state.")).map_err(OAuthServerError::OnResponse)?;
                        continue;
                    }

                    request.respond(respond_code(500, "Twitch error.")).map_err(OAuthServerError::OnResponse)?;
                    return Err(OAuthServerError::OnAuth {
                        error: String::from(error),
                        error_description: error_description.replace('+', " "),
                    });
                }
                _ => request.respond(respond_code(404, "Not found.")),
            }
            .map_err(OAuthServerError::OnResponse)?;
    }
}

fn respond_code(code: u16, description: &str) -> Response<&[u8]> {
    Response::new(
        StatusCode(code),
        vec![
            tiny_http::Header::from_bytes("Content-Type", "text/plain").unwrap(),
            tiny_http::Header::from_bytes("Cache-Control", "no-store").unwrap(),
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
    let mut state = [0; 32];
    rng.fill(&mut state)?;
    let state = state.into_iter().map(|byte| format!("{byte:x?}")).collect();
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
