use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
};
use tauri::Emitter;

use crate::error::CommandError;

mod commands;
mod error;

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
) -> Result<(), CommandError> {
    let status_event = format!("{client_id}:status");
    let log_event = format!("{client_id}:log");
    let exit_event = format!("{client_id}:exit");

    app.emit(
        &status_event,
        MinecraftStatusEvent {
            status: "starting".into(),
        },
    )
    .ok();

    let mut child = Command::new(&java_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            app.emit(
                &status_event,
                MinecraftStatusEvent {
                    status: "error".into(),
                },
            )
            .ok();
            CommandError::spawn(&java_path, e)
        })?;

    let (stdout, stderr) = match (child.stdout.take(), child.stderr.take()) {
        (Some(stdout), Some(stderr)) => (stdout, stderr),
        _ => {
            let _ = child.kill();
            app.emit(
                &status_event,
                MinecraftStatusEvent {
                    status: "error".into(),
                },
            )
            .ok();
            return Err(CommandError::launch("Нет доступа к выводу процесса Minecraft"));
        }
    };

    let app_stdout = app.clone();
    let app_stderr = app.clone();

    let log_event_stdout = log_event.clone();
    let log_event_stderr = log_event.clone();

    // stdout
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            app_stdout
                .emit(
                    &log_event_stdout,
                    MinecraftLogEvent {
                        line,
                        is_error: false,
                    },
                )
                .ok();
        }
    });

    // stderr
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            app_stderr
                .emit(
                    &log_event_stderr,
                    MinecraftLogEvent {
                        line,
                        is_error: true,
                    },
                )
                .ok();
        }
    });

    // ожидание завершения
    let app_exit = app.clone();
    let exit_event_thread = exit_event.clone();
    let status_event_thread = status_event.clone();
    thread::spawn(move || {
        let status = child.wait().ok();
        let code = status.and_then(|s| s.code());

        app_exit
            .emit(
                &status_event_thread,
                MinecraftStatusEvent {
                    status: if code == Some(0) { "exited" } else { "error" }.into(),
                },
            )
            .ok();

        app_exit.emit(&exit_event_thread, code).ok();
    });

    app.emit(
        &status_event,
        MinecraftStatusEvent {
            status: "running".into(),
        },
    )
    .ok();

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::download::DownloadRegistry::new())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            launch_minecraft,
            commands::microsoft::auth_microsoft,
            commands::microsoft::exchange_microsoft_code,
            commands::microsoft::minecraft_services_request,
            commands::microsoft::refresh_microsoft,
            commands::download::start_download,
            commands::download::await_download,
            commands::download::cancel_download,
            commands::download::list_downloads,
            commands::extract::extract_jar,
            commands::extract::extract_everything_jar,
            commands::forge::install_forge,
            commands::java::list_java,
            commands::java::probe_java,
            commands::java::finalize_java_runtime
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
