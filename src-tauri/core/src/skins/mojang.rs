use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::multipart::{Form, Part};
use reqwest::Response;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{CommandError, CommandResult};
use crate::net::http;

use super::texture::SkinVariant;

const SKINS_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const ACTIVE_SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins/active";
const ACTIVE_CAPE_URL: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";
const NAME_LOOKUP_URL: &str = "https://api.mojang.com/users/profiles/minecraft";
const SESSION_PROFILE_URL: &str = "https://sessionserver.mojang.com/session/minecraft/profile";

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub skins: Vec<Value>,
    #[serde(default)]
    pub capes: Vec<Value>,
}

pub async fn upload_skin(
    token: &str,
    bytes: Vec<u8>,
    variant: SkinVariant,
) -> CommandResult<ProfileResponse> {
    let part = Part::bytes(bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| CommandError::unknown("Не удалось собрать запрос").with_details(e.to_string()))?;

    let form = Form::new()
        .text("variant", variant.as_api())
        .part("file", part);

    let response = http::client()
        .post(SKINS_URL)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            CommandError::network("Не удалось отправить скин в Mojang").with_details(e.to_string())
        })?;

    parse(response, SKINS_URL).await
}

pub async fn reset_skin(token: &str) -> CommandResult<()> {
    let response = http::client()
        .delete(ACTIVE_SKIN_URL)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| {
            CommandError::network("Не удалось сбросить скин").with_details(e.to_string())
        })?;

    drain(response, ACTIVE_SKIN_URL).await
}

pub async fn set_cape(token: &str, cape_id: &str) -> CommandResult<ProfileResponse> {
    let response = http::client()
        .put(ACTIVE_CAPE_URL)
        .bearer_auth(token)
        .json(&json!({ "capeId": cape_id }))
        .send()
        .await
        .map_err(|e| CommandError::network("Не удалось надеть плащ").with_details(e.to_string()))?;

    parse(response, ACTIVE_CAPE_URL).await
}

pub async fn clear_cape(token: &str) -> CommandResult<()> {
    let response = http::client()
        .delete(ACTIVE_CAPE_URL)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CommandError::network("Не удалось снять плащ").with_details(e.to_string()))?;

    drain(response, ACTIVE_CAPE_URL).await
}

pub async fn download(url: &str) -> CommandResult<Vec<u8>> {
    let response = http::client()
        .get(url)
        .send()
        .await
        .map_err(|e| CommandError::network("Не удалось скачать текстуру").with_details(e.to_string()))?;

    let status = response.status();

    if !status.is_success() {
        return Err(http::http_status_error(status, url));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| CommandError::download("Текстура скачалась не полностью").with_details(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSkin {
    pub name: String,
    pub url: String,
    pub variant: SkinVariant,
}

pub async fn player_skin(name: &str) -> CommandResult<PlayerSkin> {
    let name = name.trim();

    if name.is_empty() {
        return Err(CommandError::fs("Укажите никнейм"));
    }

    let lookup_url = format!("{NAME_LOOKUP_URL}/{name}");

    let response = http::client()
        .get(&lookup_url)
        .send()
        .await
        .map_err(|e| CommandError::network("Не удалось найти игрока").with_details(e.to_string()))?;

    if response.status().as_u16() == 404 || response.status().as_u16() == 204 {
        return Err(CommandError::fs(format!("Игрок {name} не найден")));
    }

    let profile: Value = parse(response, &lookup_url).await?;

    let id = profile
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| CommandError::manifest("Mojang вернул игрока без идентификатора"))?;

    let session_url = format!("{SESSION_PROFILE_URL}/{id}");

    let session: Value = http::client()
        .get(&session_url)
        .send()
        .await
        .map_err(|e| CommandError::network("Не удалось получить профиль игрока").with_details(e.to_string()))?
        .json()
        .await
        .map_err(|e| CommandError::manifest("Некорректный ответ сессии").with_details(e.to_string()))?;

    let encoded = session
        .get("properties")
        .and_then(Value::as_array)
        .and_then(|properties| {
            properties
                .iter()
                .find(|property| property.get("name").and_then(Value::as_str) == Some("textures"))
        })
        .and_then(|property| property.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| CommandError::manifest("У игрока нет текстур"))?;

    let decoded = STANDARD
        .decode(encoded)
        .map_err(|e| CommandError::manifest("Текстуры игрока не читаются").with_details(e.to_string()))?;

    let textures: Value = serde_json::from_slice(&decoded)
        .map_err(|e| CommandError::manifest("Текстуры игрока не читаются").with_details(e.to_string()))?;

    let skin = textures
        .get("textures")
        .and_then(|textures| textures.get("SKIN"))
        .ok_or_else(|| CommandError::fs(format!("У игрока {name} стандартный скин")))?;

    let url = skin
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| CommandError::manifest("У текстуры нет ссылки"))?;

    let model = skin
        .get("metadata")
        .and_then(|metadata| metadata.get("model"))
        .and_then(Value::as_str);

    Ok(PlayerSkin {
        name: profile
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(name)
            .to_string(),
        url: url.to_string(),
        variant: SkinVariant::from_model(model),
    })
}

async fn parse<T: serde::de::DeserializeOwned>(response: Response, url: &str) -> CommandResult<T> {
    let status = response.status();

    if !status.is_success() {
        return Err(failure(response, url).await);
    }

    response.json().await.map_err(|e| {
        CommandError::manifest(format!("Некорректный ответ: {url}")).with_details(e.to_string())
    })
}

async fn drain(response: Response, url: &str) -> CommandResult<()> {
    if response.status().is_success() {
        return Ok(());
    }

    Err(failure(response, url).await)
}

async fn failure(response: Response, url: &str) -> CommandError {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();

    match status.as_u16() {
        401 | 403 => CommandError::auth_expired("Сессия Minecraft недействительна")
            .with_details(format!("HTTP {status}\n{text}")),
        404 => CommandError::auth("На этом аккаунте Microsoft нет купленного Minecraft")
            .with_details(format!("HTTP {status}\n{url}\n{text}")),
        // Mojang жёстко ограничивает частоту смены внешнего вида.
        429 => CommandError::network("Mojang просит подождать: слишком часто меняете внешний вид")
            .with_details(format!("HTTP {status}\n{text}")),
        _ => CommandError::network(format!("Сервер ответил HTTP {status}"))
            .with_details(format!("{url}\n{text}")),
    }
}
