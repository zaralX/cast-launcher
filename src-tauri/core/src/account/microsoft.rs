use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use reqwest::Response;

use crate::error::{CommandError, CommandResult};
use crate::net::http;

pub const CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";
pub const REDIRECT_URI: &str = "http://localhost:55325/";
pub const LISTEN_ADDR: &str = "127.0.0.1:55325";

const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
const XBOX_LIVE_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

pub fn authorize_url(code_challenge: &str, state: &str) -> String {
    let query = [
        ("client_id", CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", REDIRECT_URI),
        ("scope", "XboxLive.SignIn XboxLive.offline_access"),
        ("code_challenge", code_challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
    ]
    .iter()
    .map(|(key, value)| format!("{key}={}", urlencode(value)))
    .collect::<Vec<_>>()
    .join("&");

    format!("https://login.live.com/oauth20_authorize.srf?{query}")
}

fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }

    out
}

#[derive(Debug, Clone, Deserialize)]
pub struct MicrosoftTokens {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XboxTokens {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    #[serde(default)]
    pub display_claims: Option<DisplayClaims>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DisplayClaims {
    #[serde(default)]
    pub xui: Vec<XuiClaim>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XuiClaim {
    #[serde(default)]
    pub uhs: Option<String>,
}

impl XboxTokens {
    pub fn user_hash(&self) -> Option<&str> {
        self.display_claims
            .as_ref()?
            .xui
            .first()?
            .uhs
            .as_deref()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MinecraftToken {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<Value>,
    #[serde(default)]
    pub capes: Vec<Value>,
}

pub async fn exchange_code(code: &str, code_verifier: &str) -> CommandResult<MicrosoftTokens> {
    request_token(&[
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", code_verifier),
    ])
    .await
}

pub async fn refresh(refresh_token: &str) -> CommandResult<MicrosoftTokens> {
    request_token(&[
        ("grant_type", "refresh_token"),
        ("client_id", CLIENT_ID),
        ("refresh_token", refresh_token),
        ("redirect_uri", REDIRECT_URI),
    ])
    .await
}

async fn request_token(params: &[(&str, &str)]) -> CommandResult<MicrosoftTokens> {
    let response = http::client()
        .post(TOKEN_URL)
        .form(params)
        .send()
        .await
        .map_err(|e| {
            CommandError::network("Не удалось связаться с сервером Microsoft")
                .with_details(e.to_string())
        })?;

    let status = response.status();
    let json: Value = response.json().await.map_err(|e| {
        CommandError::auth(format!("Некорректный ответ Microsoft (HTTP {status})"))
            .with_details(e.to_string())
    })?;

    if let Some(error) = json.get("error").and_then(Value::as_str) {
        let description = json
            .get("error_description")
            .and_then(Value::as_str)
            .unwrap_or(error);

        return Err(
            CommandError::auth(format!("Microsoft отклонил запрос: {error}")).with_details(description)
        );
    }

    serde_json::from_value(json).map_err(|e| {
        CommandError::auth("Microsoft вернул ответ без токенов").with_details(e.to_string())
    })
}

pub async fn xbox_live(microsoft_access_token: &str) -> CommandResult<XboxTokens> {
    post_json(
        XBOX_LIVE_URL,
        &json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={microsoft_access_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
        None,
    )
    .await
}

pub async fn xsts(xbox_token: &str) -> CommandResult<XboxTokens> {
    post_json(
        XSTS_URL,
        &json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbox_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
        None,
    )
    .await
}

pub async fn minecraft_login(user_hash: &str, xsts_token: &str) -> CommandResult<MinecraftToken> {
    post_json(
        MINECRAFT_LOGIN_URL,
        &json!({ "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}") }),
        None,
    )
    .await
}

pub async fn profile(minecraft_access_token: &str) -> CommandResult<MinecraftProfile> {
    let response = http::client()
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(minecraft_access_token)
        .send()
        .await
        .map_err(|e| {
            CommandError::network("Не удалось получить профиль Minecraft").with_details(e.to_string())
        })?;

    parse(response, MINECRAFT_PROFILE_URL).await
}

async fn post_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &Value,
    bearer: Option<&str>,
) -> CommandResult<T> {
    let mut request = http::client().post(url).json(body);

    if let Some(token) = bearer {
        request = request.bearer_auth(token);
    }

    let response = request.send().await.map_err(|e| {
        CommandError::network(format!("Запрос не выполнен: {url}")).with_details(e.to_string())
    })?;

    parse(response, url).await
}

async fn parse<T: serde::de::DeserializeOwned>(response: Response, url: &str) -> CommandResult<T> {
    let status = response.status();

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();

        return Err(match status.as_u16() {
            401 | 403 => CommandError::auth_expired("Сессия Minecraft недействительна")
                .with_details(format!("HTTP {status}\n{text}")),
            404 => CommandError::auth("На этом аккаунте Microsoft нет купленного Minecraft")
                .with_details(format!("HTTP {status}\n{url}\n{text}")),
            _ => CommandError::network(format!("Сервер ответил HTTP {status}"))
                .with_details(format!("{url}\n{text}")),
        });
    }

    response.json().await.map_err(|e| {
        CommandError::manifest(format!("Некорректный ответ: {url}")).with_details(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_encodes_scope_and_state() {
        let url = authorize_url("challenge-123", "state-456");

        assert!(url.contains("client_id=c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb"));
        assert!(url.contains("scope=XboxLive.SignIn%20XboxLive.offline_access"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A55325%2F"));
        assert!(url.contains("code_challenge=challenge-123"));
        assert!(url.contains("state=state-456"));
    }

    #[test]
    fn user_hash_is_read_from_display_claims() {
        let tokens: XboxTokens = serde_json::from_value(json!({
            "Token": "t",
            "DisplayClaims": { "xui": [{ "uhs": "hash" }] }
        }))
        .unwrap();

        assert_eq!(tokens.user_hash(), Some("hash"));

        let empty: XboxTokens = serde_json::from_value(json!({ "Token": "t" })).unwrap();
        assert_eq!(empty.user_hash(), None);
    }

    #[test]
    fn urlencode_leaves_unreserved_characters() {
        assert_eq!(urlencode("abcXYZ019-_.~"), "abcXYZ019-_.~");
        assert_eq!(urlencode("a b/c"), "a%20b%2Fc");
    }
}
