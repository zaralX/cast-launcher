use tauri::Manager;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
};
use tauri::Emitter;

mod commands;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(serde::Serialize, Clone)]
struct MinecraftLogEvent {
    line: String,
    is_error: bool,
}

#[derive(serde::Serialize, Clone)]
struct MinecraftStatusEvent {
    status: String, // starting | running | exited | error
}


#[tauri::command]
fn launch_minecraft(
    app: tauri::AppHandle,
    java_path: String,
    client_id: String,
    args: Vec<String>,
) -> Result<(), String> {
    let mut child = Command::new(java_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let status_event = format!("{client_id}:status");
    let log_event = format!("{client_id}:log");
    let exit_event = format!("{client_id}:exit");

    app.emit(
        &status_event,
        MinecraftStatusEvent {
            status: "starting".into(),
        },
    ).ok();

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let app_stdout = app.clone();
    let app_stderr = app.clone();

    let log_event_stdout = log_event.clone();
    let log_event_stderr = log_event.clone();

    // stdout
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            app_stdout.emit(
                &log_event_stdout,
                MinecraftLogEvent {
                    line,
                    is_error: false,
                },
            ).ok();
        }
    });

    // stderr
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            app_stderr.emit(
                &log_event_stderr,
                MinecraftLogEvent {
                    line,
                    is_error: true,
                },
            ).ok();
        }
    });

    // ожидание завершения
    let app_exit = app.clone();
    let exit_event_thread = exit_event.clone();
    thread::spawn(move || {
        let status = child.wait().ok();
        app_exit.emit(
            &exit_event_thread,
            status.map(|s| s.code()).unwrap_or(None),
        ).ok();
    });

    app.emit(
        &status_event,
        MinecraftStatusEvent {
            status: "running".into(),
        },
    ).ok();

    Ok(())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            launch_minecraft,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
