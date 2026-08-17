use std::sync::Arc;

use tauri::{AppHandle, Manager, WebviewWindowBuilder};

use cast_core::config::AfterLaunch;

use crate::state::AppState;

pub const MAIN: &str = "main";

pub fn supervising(app: &AppHandle) -> bool {
    app.try_state::<Arc<AppState>>()
        .map(|state| state.processes.busy())
        .unwrap_or(false)
}

pub fn restore(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };

    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

pub fn apply_after_launch(app: &AppHandle, mode: AfterLaunch) {
    for window in app.webview_windows().values() {
        let hidden = match mode {
            AfterLaunch::Nothing => return,
            AfterLaunch::Hide => window.hide(),
            AfterLaunch::Close => window.close(),
        };

        if let Err(error) = hidden {
            eprintln!("Не удалось убрать окно лаунчера: {error}");
        }
    }
}

pub fn focus_or_create(app: &AppHandle) {
    if app.get_webview_window(MAIN).is_some() {
        restore(app);
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
