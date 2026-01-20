use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::Emitter;
use zip::ZipArchive;

#[tauri::command]
pub async fn install_forge(
    app: tauri::AppHandle,
    java_path: String,
    installer_path: String,
    minecraft_dir: String,
) -> Result<(), String> {
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
            .map_err(|e| e.to_string())?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let out_reader = BufReader::new(stdout);
        let err_reader = BufReader::new(stderr);

        for line in out_reader.lines() {
            if let Ok(line) = line {
                let _ = app.emit("forgeinstaller-log", line);
            }
        }

        for line in err_reader.lines() {
            if let Ok(line) = line {
                let _ = app.emit("forgeinstaller-error", line);
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;

        if !status.success() {
            return Err("Forge installer завершился с ошибкой".into());
        }

        Ok(())
    })
        .await
        .map_err(|e| e.to_string())?
}