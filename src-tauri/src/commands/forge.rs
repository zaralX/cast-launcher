use crate::error::CommandError;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tauri::Emitter;

const TAIL_LINES: usize = 40;

type OutputSink = Arc<Mutex<Vec<String>>>;

fn spawn_reader<R: Read + Send + 'static>(
    app: tauri::AppHandle,
    reader: R,
    event: &'static str,
    sink: OutputSink,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            let _ = app.emit(event, line.clone());
            if let Ok(mut sink) = sink.lock() {
                sink.push(line);
            }
        }
    })
}

fn tail(sink: &OutputSink) -> String {
    let lines = match sink.lock() {
        Ok(lines) => lines,
        Err(poisoned) => poisoned.into_inner(),
    };
    let from = lines.len().saturating_sub(TAIL_LINES);
    lines[from..].join("\n")
}

#[tauri::command]
pub async fn install_forge(
    app: tauri::AppHandle,
    java_path: String,
    installer_path: String,
    minecraft_dir: String,
) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut child = Command::new(&java_path)
            .arg("-jar")
            .arg(&installer_path)
            .arg("--installClient")
            .arg(&minecraft_dir)
            .current_dir(&minecraft_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CommandError::spawn(&java_path, e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CommandError::forge("Нет доступа к выводу установщика Forge"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CommandError::forge("Нет доступа к выводу установщика Forge"))?;

        let sink: OutputSink = Arc::new(Mutex::new(Vec::new()));

        let out_reader = spawn_reader(app.clone(), stdout, "forgeinstaller-log", Arc::clone(&sink));
        let err_reader = spawn_reader(app.clone(), stderr, "forgeinstaller-error", Arc::clone(&sink));

        let status = child.wait().map_err(|e| {
            CommandError::forge("Установщик Forge завершился аварийно").with_details(e.to_string())
        })?;

        let _ = out_reader.join();
        let _ = err_reader.join();

        if !status.success() {
            let code = status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "неизвестен".into());

            return Err(
                CommandError::forge(format!("Установщик Forge завершился с кодом {code}"))
                    .with_details(tail(&sink)),
            );
        }

        Ok(())
    })
    .await
    .map_err(|e| {
        CommandError::forge("Задача установки Forge прервана").with_details(e.to_string())
    })?
}
