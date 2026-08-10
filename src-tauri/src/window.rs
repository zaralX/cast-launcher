use std::sync::Arc;

use tauri::{AppHandle, Manager, WebviewWindowBuilder};

use crate::state::AppState;

pub const MAIN: &str = "main";

pub fn supervising(app: &AppHandle) -> bool {
    app.try_state::<Arc<AppState>>()
        .map(|state| state.processes.has_running())
        .unwrap_or(false)
}

pub fn focus_or_create(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN)
        .cloned();

    let Some(config) = config else {
        eprintln!("В конфигурации нет окна «{MAIN}», открыть лаунчер не получится");
        return;
    };

    let built = WebviewWindowBuilder::from_config(app, &config).and_then(|builder| builder.build());

    if let Err(error) = built {
        eprintln!("Не удалось открыть окно лаунчера: {error}");
    }
}

pub fn exit_if_idle(app: &AppHandle) {
    if supervising(app) || !app.webview_windows().is_empty() {
        return;
    }

    app.exit(0);
}
