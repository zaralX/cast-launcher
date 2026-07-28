use std::collections::HashMap;
use std::time::Duration;

use sha1::Digest;
use tiny_http::{Response, Server};
use url::Url;

use crate::error::{CommandError, CommandResult};

use super::microsoft;

const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

const SUCCESS_PAGE: &str = "<!doctype html><meta charset=\"utf-8\">\
<title>Cast Launcher</title>\
<body style=\"font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\">\
<h1>Готово, окно можно закрыть</h1>";

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn generate() -> Self {
        let verifier = random_token(64);
        let digest = sha2_256(verifier.as_bytes());

        Self {
            challenge: base64_url(&digest),
            verifier,
        }
    }
}

pub async fn login(open_browser: impl FnOnce(&str) -> CommandResult<()>) -> CommandResult<super::Account> {
    let pkce = Pkce::generate();
    let state = random_token(24);

    let server = Server::http(microsoft::LISTEN_ADDR).map_err(|e| {
        CommandError::port_busy(format!(
            "Не удалось занять {} для входа",
            microsoft::LISTEN_ADDR
        ))
        .with_details(e.to_string())
    })?;

    open_browser(&microsoft::authorize_url(&pkce.challenge, &state))?;

    let expected_state = state.clone();
    let code = tokio::task::spawn_blocking(move || wait_for_code(server, &expected_state))
        .await
        .map_err(|e| CommandError::task_panicked("ожидание ответа Microsoft", e))??;

    let tokens = microsoft::exchange_code(&code, &pkce.verifier).await?;

    super::complete_login(tokens).await
}

fn wait_for_code(server: Server, expected_state: &str) -> CommandResult<String> {
    let deadline = std::time::Instant::now() + LOGIN_TIMEOUT;

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(CommandError::auth("Вход не был завершён вовремя"));
        }

        let Ok(Some(request)) = server.recv_timeout(remaining) else {
            return Err(CommandError::auth("Соединение с браузером не установлено"));
        };

        let query = parse_query(request.url());

        if query.is_empty() {
            let _ = request.respond(Response::from_string("").with_status_code(204));
            continue;
        }

        let outcome = interpret(&query, expected_state);

        let _ = request.respond(match &outcome {
            Ok(_) => Response::from_string(SUCCESS_PAGE).with_header(html_header()),
            Err(error) => Response::from_string(format!("<h1>Ошибка входа</h1><p>{}</p>", error.message))
                .with_header(html_header())
                .with_status_code(400),
        });

        return outcome;
    }
}

fn interpret(query: &HashMap<String, String>, expected_state: &str) -> CommandResult<String> {
    if let Some(error) = query.get("error") {
        let description = query.get("error_description").unwrap_or(error);
        return Err(CommandError::auth(description.clone()));
    }

    if query.get("state").map(String::as_str) != Some(expected_state) {
        return Err(CommandError::auth("Ответ Microsoft не соответствует запросу входа"));
    }

    query
        .get("code")
        .cloned()
        .ok_or_else(|| CommandError::auth("Microsoft не вернул код авторизации"))
}

fn parse_query(request_url: &str) -> HashMap<String, String> {
    let Ok(url) = Url::parse(&format!("http://localhost{request_url}")) else {
        return HashMap::new();
    };

    url.query_pairs().into_owned().collect()
}

fn html_header() -> tiny_http::Header {
    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
        .expect("корректный заголовок")
}

fn random_token(length: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

    let mut token = String::with_capacity(length);

    while token.len() < length {
        for byte in uuid::Uuid::new_v4().as_bytes() {
            token.push(ALPHABET[*byte as usize % ALPHABET.len()] as char);
            if token.len() == length {
                break;
            }
        }
    }

    token
}

fn sha2_256(input: &[u8]) -> Vec<u8> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

fn base64_url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(ALPHABET[(triple >> 18 & 0x3f) as usize] as char);
        out.push(ALPHABET[(triple >> 12 & 0x3f) as usize] as char);

        if chunk.len() > 1 {
            out.push(ALPHABET[(triple >> 6 & 0x3f) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(triple & 0x3f) as usize] as char);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_url_safe_base64_of_sha256() {
        let pkce = Pkce::generate();

        assert_eq!(pkce.verifier.len(), 64);
        assert_eq!(pkce.challenge.len(), 43, "SHA-256 без паддинга даёт 43 символа");
        assert!(!pkce.challenge.contains(['+', '/', '=']));
    }

    #[test]
    fn pkce_pairs_are_unique() {
        assert_ne!(Pkce::generate().verifier, Pkce::generate().verifier);
    }

    #[test]
    fn base64_url_matches_known_vectors() {
        assert_eq!(base64_url(b""), "");
        assert_eq!(base64_url(b"f"), "Zg");
        assert_eq!(base64_url(b"fo"), "Zm8");
        assert_eq!(base64_url(b"foo"), "Zm9v");
        assert_eq!(base64_url(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn query_is_parsed_from_redirect_path() {
        let query = parse_query("/?code=abc&state=xyz");

        assert_eq!(query.get("code").map(String::as_str), Some("abc"));
        assert_eq!(query.get("state").map(String::as_str), Some("xyz"));
        assert!(parse_query("/favicon.ico").is_empty());
    }

    #[test]
    fn mismatched_state_is_rejected() {
        let query = parse_query("/?code=abc&state=wrong");
        assert!(interpret(&query, "expected").is_err());

        let good = parse_query("/?code=abc&state=expected");
        assert_eq!(interpret(&good, "expected").unwrap(), "abc");
    }

    #[test]
    fn error_response_is_surfaced() {
        let query = parse_query("/?error=access_denied&error_description=User+cancelled&state=expected");
        let error = interpret(&query, "expected").unwrap_err();

        assert_eq!(error.message, "User cancelled");
    }
}
