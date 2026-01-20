use std::collections::HashMap;
use tiny_http::Server;
use url::Url;
use std::thread;
use reqwest::Client;
use serde_json::Value;
use tauri::AppHandle;
use tauri::Emitter;
use tauri_plugin_http::reqwest;

pub fn start_oauth_listener(
    on_code: impl Fn(String) + Send + 'static
) {
    thread::spawn(move || {
        let server = Server::http("127.0.0.1:55325")
            .expect("port 55325 busy");

        if let Ok(request) = server.recv() {
            let url = format!("http://localhost{}", request.url());
            let parsed = Url::parse(&url).unwrap();

            let code = parsed
                .query_pairs()
                .find(|(k, _)| k == "code")
                .map(|(_, v)| v.to_string());

            let response = tiny_http::Response::from_string(
                "<h1>Можно закрыть окно</h1>"
            );

            let _ = request.respond(response);

            if let Some(code) = code {
                on_code(code);
            }
        }
    });
}

#[tauri::command]
pub fn auth_microsoft(app: AppHandle) {
    start_oauth_listener(move |code| {
        println!("OAuth code: {}", code);
        let _ = app.emit(
            "microsoft-oauth-code",
            code
        );
    });
}

#[tauri::command]
pub async fn exchange_microsoft_code(client_id: String, code: String, code_verifier: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", &client_id),
        ("code", &code),
        ("redirect_uri", "http://localhost:55325/"),
        ("code_verifier", &code_verifier)
    ];

    let resp = client.post("https://login.live.com/oauth20_token.srf")
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json)
}

#[tauri::command]
pub async fn minecraft_services_request(
    url: String,
    method: Option<String>, // "GET" или "POST", по умолчанию GET
    body: Option<Value>,
    headers: Option<HashMap<String, String>>,
) -> Result<Value, String> {
    let client = Client::new();
    let method = method.unwrap_or_else(|| "GET".to_string()).to_uppercase();

    // Создаём request в зависимости от метода
    let mut request = match method.as_str() {
        "POST" => client.post(&url),
        "GET" => client.get(&url),
        other => return Err(format!("Unsupported HTTP method: {}", other)),
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
    let res = request
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    // Проверка успешности
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("HTTP error {}: {}", status, text));
    }

    // Парсинг ответа
    let json: Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(json)
}