use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use serde_json::json;
use tauri::{AppHandle, Runtime};
use tauri_plugin_aptabase::{EventTracker, InitOptions};

pub use cast_core::telemetry::{classify_crash, host_of, megabytes, minutes, Event};

use cast_core::config::AppConfig;

use crate::state::AppState;

const HOST: &str = "https://telemetry.zaralx.ru";
const APP_KEY: &str = "A-SH-6557538081";

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    STARTED_AT.get_or_init(Instant::now);

    tauri_plugin_aptabase::Builder::new(app_key())
        .with_options(InitOptions {
            host: Some(HOST.into()),
            flush_interval: None,
        })
        .with_panic_hook(Box::new(|client, info, message| {
            let location = info
                .location()
                .map(|location| format!("{}:{}", location.file(), location.line()))
                .unwrap_or_default();

            if ENABLED.load(Ordering::Relaxed) {
                let _ = client.track_event(
                    "panic",
                    Some(json!({ "at": location, "message": trimmed(&message) })),
                );
            }
        }))
        .build()
}

fn app_key() -> &'static str {
    match option_env!("CAST_APTABASE_KEY") {
        Some(key) if !key.is_empty() => key,
        _ => APP_KEY,
    }
}

fn trimmed(message: &str) -> String {
    message.chars().take(cast_core::telemetry::MAX_VALUE_LEN).collect()
}

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn track(app: &AppHandle, event: Event) {
    if !is_enabled() {
        return;
    }

    send(app, event);
}

pub fn track_always(app: &AppHandle, event: Event) {
    send(app, event);
}

fn send(app: &AppHandle, event: Event) {
    let name = event.name().to_string();

    if let Err(error) = app.track_event(&name, Some(event.into_props())) {
        eprintln!("Событие телеметрии «{name}» не ушло: {error}");
    }
}

pub fn uptime_seconds() -> f64 {
    STARTED_AT
        .get()
        .map(|started| started.elapsed().as_secs_f64())
        .unwrap_or(0.0)
}

pub async fn app_started(app: &AppHandle, state: &Arc<AppState>) {
    let config = state.config().await;
    let instances = state.instances.all().await;
    let accounts = state.accounts.config().await;

    let microsoft = accounts
        .accounts
        .iter()
        .filter(|account| account.user_type() == "msa")
        .count();

    let event = Event::new("app_started")
        .text("version", env!("CARGO_PKG_VERSION"))
        .text("arch", std::env::consts::ARCH)
        .num("cpu_cores", cpu_cores())
        .num("ram_mb", total_ram_mb())
        .num("instances", instances.len() as f64)
        .num("installed", instances.iter().filter(|item| item.installed).count() as f64)
        .num("accounts", accounts.accounts.len() as f64)
        .num("microsoft_accounts", microsoft as f64)
        .num("playtime_h", total_playtime_hours(&instances))
        .text("java_mode", config.java.java_mode.key())
        .num("min_ram", config.java.min_ram)
        .num("max_ram", config.java.max_ram)
        .text("theme", &config.launcher.theme)
        .text("accent", &config.launcher.accent)
        .text("language", &config.launcher.language)
        .flag("compact", config.launcher.compact)
        .flag("auto_update", config.launcher.auto_update)
        .flag("custom_dir", custom_dir(state, &config).await)
        .flag("custom_catalog", !config.launcher.castpack_url.trim().is_empty());

    track(app, event);
}

pub fn app_exited(app: &AppHandle) {
    track(
        app,
        Event::new("app_exited").num("session_min", uptime_seconds() / 60.0),
    );

    app.flush_events_blocking();
}

pub fn settings_changed(app: &AppHandle, before: &AppConfig, after: &AppConfig) {
    let mut changes: Vec<(&str, String)> = Vec::new();

    let mut text = |key: &'static str, before: &str, after: &str| {
        if before != after {
            changes.push((key, after.to_string()));
        }
    };

    text("language", &before.launcher.language, &after.launcher.language);
    text("theme", &before.launcher.theme, &after.launcher.theme);
    text("accent", &before.launcher.accent, &after.launcher.accent);
    text(
        "java_mode",
        before.java.java_mode.key(),
        after.java.java_mode.key(),
    );

    let mut flag = |key: &'static str, before: bool, after: bool| {
        if before != after {
            changes.push((key, u8::from(after).to_string()));
        }
    };

    flag("compact", before.launcher.compact, after.launcher.compact);
    flag("auto_update", before.launcher.auto_update, after.launcher.auto_update);
    flag("telemetry", before.launcher.telemetry, after.launcher.telemetry);
    flag(
        "custom_catalog",
        !before.launcher.castpack_url.trim().is_empty(),
        !after.launcher.castpack_url.trim().is_empty(),
    );
    if before.launcher.dir.trim() != after.launcher.dir.trim() {
        changes.push(("launcher_dir", "changed".to_string()));
    }

    let mut number = |key: &'static str, before: u32, after: u32| {
        if before != after {
            changes.push((key, after.to_string()));
        }
    };

    number("min_ram", before.java.min_ram, after.java.min_ram);
    number("max_ram", before.java.max_ram, after.java.max_ram);

    for (key, value) in changes {
        let event = Event::new("settings_changed").text("key", key).text("value", value);

        match key == "telemetry" {
            true => track_always(app, event),
            false => track(app, event),
        }
    }
}

async fn custom_dir(state: &Arc<AppState>, config: &AppConfig) -> bool {
    let paths = state.paths().await;

    paths.root() != paths.config_root() && !config.launcher.dir.trim().is_empty()
}

fn total_playtime_hours(instances: &[cast_core::instance::Instance]) -> f64 {
    let seconds: u64 = instances
        .iter()
        .map(|instance| instance.playtime.total_seconds)
        .sum();

    seconds as f64 / 3600.0
}

fn cpu_cores() -> f64 {
    std::thread::available_parallelism()
        .map(|cores| cores.get() as f64)
        .unwrap_or(0.0)
}

fn total_ram_mb() -> f64 {
    let system = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_memory(sysinfo::MemoryRefreshKind::nothing().with_ram()),
    );

    megabytes(system.total_memory())
}
