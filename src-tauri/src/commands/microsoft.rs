use crate::error::CommandError;
use serde_json::Value;
use std::collections::HashMap;
use std::thread;
use tauri::AppHandle;
use tauri::Emitter;
use tauri_plugin_http::reqwest;
use tiny_http::Server;
use url::Url;

const REDIRECT_URI: &str = "http://localhost:55325/";
const LISTEN_ADDR: &str = "127.0.0.1:55325";

fn start_oauth_listener(app: AppHandle) -> Result<(), CommandError> {
    let server = Server::http(LISTEN_ADDR).map_err(|e| {
        CommandError::port_busy(format!("Не удалось занять {LISTEN_ADDR} для входа"))
            .with_details(e.to_string())
    })?;

    thread::spawn(move || {
        let Ok(request) = server.recv() else {
            let _ = app.emit("microsoft-oauth-error", "Соединение с браузером не установлено");
            return;
        };

        let redirect = format!("http://localhost{}", request.url());

        let _ = request.respond(tiny_http::Response::from_string(
            "<h1>Можно закрыть окно</h1>",
        ));

        let Ok(parsed) = Url::parse(&redirect) else {
            let _ = app.emit("microsoft-oauth-error", "Некорректный ответ от Microsoft");
            return;
        };

        let query: HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        if let Some(error) = query.get("error") {
            let description = query
                .get("error_description")
                .cloned()
                .unwrap_or_else(|| error.clone());
            let _ = app.emit("microsoft-oauth-error", description);
            return;
        }

        match query.get("code") {
            Some(code) => {
                let _ = app.emit("microsoft-oauth-code", code.clone());
            }
            None => {
                let _ = app.emit("microsoft-oauth-error", "Microsoft не вернул код авторизации");
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn auth_microsoft(app: AppHandle) -> Result<(), CommandError> {
    start_oauth_listener(app)
}

fn token_error(json: &Value) -> Option<CommandError> {
    let error = json.get("error")?.as_str()?;
    let description = json
        .get("error_description")
        .and_then(|v| v.as_str())
        .unwrap_or(error);

    Some(CommandError::auth(format!("Microsoft отклонил запрос: {error}")).with_details(description))
}

async fn request_token(params: &[(&str, &str)]) -> Result<Value, CommandError> {
    let response = reqwest::Client::new()
        .post("https://login.live.com/oauth20_token.srf")
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

    if let Some(error) = token_error(&json) {
        return Err(error);
    }

    Ok(json)
}

#[tauri::command]
pub async fn exchange_microsoft_code(
    client_id: String,
    code: String,
    code_verifier: String,
) -> Result<Value, CommandError> {
    request_token(&[
        ("grant_type", "authorization_code"),
        ("client_id", &client_id),
        ("code", &code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", &code_verifier),
    ])
    .await
}

#[tauri::command]
pub async fn refresh_microsoft(
    client_id: String,
    refresh_token: String,
) -> Result<Value, CommandError> {
    request_token(&[
        ("grant_type", "refresh_token"),
        ("client_id", &client_id),
        ("refresh_token", &refresh_token),
        ("redirect_uri", REDIRECT_URI),
    ])
    .await
}

#[tauri::command]
pub async fn minecraft_services_request(
    url: String,
    method: Option<String>, // "GET" или "POST", по умолчанию GET
    body: Option<Value>,
    headers: Option<HashMap<String, String>>,
) -> Result<Value, CommandError> {
    let client = reqwest::Client::new();
    let method = method.unwrap_or_else(|| "GET".to_string()).to_uppercase();

    // Создаём request в зависимости от метода
    let mut request = match method.as_str() {
        "POST" => client.post(&url),
        "GET" => client.get(&url),
        other => {
            return Err(CommandError::new(
                "UNKNOWN",
                format!("Неподдерживаемый HTTP-метод: {other}"),
            ))
        }
    };

    // Добавляем заголовки, если они есть
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(&key, &value);
        }
    }

    // Добавляем Content-Type и body только для POST с body
    if method == "POST" {
        if let Some(json_body) = body {
            request = request
                .header("Content-Type", "application/json")
                .json(&json_body);
        }
    }

    // Отправка запроса
    let response = request.send().await.map_err(|e| {
        CommandError::network(format!("Запрос не выполнен: {url}")).with_details(e.to_string())
    })?;

    // Проверка успешности
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();

        if status == 401 || status == 403 {
            return Err(CommandError::new(
                "AUTH_EXPIRED",
                "Сессия Minecraft недействительна",
            )
            .with_details(format!("HTTP {status}\n{text}")));
        }

        if status == 404 {
            return Err(CommandError::auth(
                "На этом аккаунте Microsoft нет купленного Minecraft",
            )
            .with_details(format!("HTTP {status}\n{url}\n{text}")));
        }

        return Err(CommandError::network(format!("Сервер ответил HTTP {status}"))
            .with_details(format!("{url}\n{text}")));
    }

    // Парсинг ответа
    response.json().await.map_err(|e| {
        CommandError::new("MANIFEST_INVALID", format!("Некорректный ответ: {url}"))
            .with_details(e.to_string())
    })
}
