use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{ensure_dir, merge_dir, remove_dir_if_exists, write_json_atomic};
use crate::paths::{ForgePaths, LauncherPaths};

const TAIL_LINES: usize = 40;

pub async fn install(
    paths: &LauncherPaths,
    java_path: &str,
    forge_version: &str,
    installer_jar: &Path,
    on_log: impl Fn(&str) + Send + Sync,
) -> CommandResult<()> {
    let cache = paths.forge_cache(forge_version);
    let workspace = paths.scratch("forge");

    let result = run_in_workspace(
        paths,
        java_path,
        forge_version,
        installer_jar,
        &workspace,
        &cache,
        on_log,
    )
    .await;

    remove_dir_if_exists(&workspace).await;

    result
}

async fn run_in_workspace(
    paths: &LauncherPaths,
    java_path: &str,
    forge_version: &str,
    installer_jar: &Path,
    workspace: &Path,
    cache: &ForgePaths,
    on_log: impl Fn(&str) + Send + Sync,
) -> CommandResult<()> {
    ensure_dir(workspace).await?;
    ensure_dir(cache.root()).await?;

    write_json_atomic(
        &workspace.join("launcher_profiles.json"),
        &serde_json::json!({
            "profiles": {},
            "clientToken": "00000000-0000-0000-0000-000000000000",
            "launcherVersion": { "name": "cast-launcher", "format": 21 }
        }),
    )
    .await?;

    let libraries = paths.libraries();
    let workspace_libraries = workspace.join("libraries");
    ensure_dir(&libraries).await?;
    let linked = link_libraries(&libraries, &workspace_libraries).await;

    let tail = run_installer(java_path, installer_jar, workspace, on_log).await?;

    if !linked {
        merge_dir(&workspace_libraries, &libraries).await?;
    }

    let produced = collect_output(workspace, forge_version).await.ok_or_else(|| {
        CommandError::forge("Установщик Forge не создал ожидаемые файлы").with_details(tail)
    })?;

    tokio::fs::copy(&produced.client_jar, cache.client_jar())
        .await
        .map_err(|e| CommandError::io("Не удалось сохранить клиент Forge", &produced.client_jar, e))?;

    tokio::fs::copy(&produced.client_json, cache.client_json())
        .await
        .map_err(|e| CommandError::io("Не удалось сохранить манифест Forge", &produced.client_json, e))?;

    Ok(())
}

async fn run_installer(
    java_path: &str,
    installer_jar: &Path,
    workspace: &Path,
    on_log: impl Fn(&str) + Send + Sync,
) -> CommandResult<String> {
    let mut child = new_command(java_path)
        .arg("-jar")
        .arg(installer_jar)
        .arg("--installClient")
        .arg(workspace)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CommandError::spawn(java_path, e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut tail: Vec<String> = Vec::new();

    let mut stdout_lines = stdout.map(|out| BufReader::new(out).lines());
    let mut stderr_lines = stderr.map(|err| BufReader::new(err).lines());

    loop {
        tokio::select! {
            line = next_line(&mut stdout_lines) => match line {
                Some(line) => push_tail(&mut tail, &line, &on_log),
                None => stdout_lines = None,
            },
            line = next_line(&mut stderr_lines) => match line {
                Some(line) => push_tail(&mut tail, &line, &on_log),
                None => stderr_lines = None,
            },
            status = child.wait(), if stdout_lines.is_none() && stderr_lines.is_none() => {
                let status = status.map_err(|e| {
                    CommandError::forge("Установщик Forge завершился аварийно").with_details(e.to_string())
                })?;

                let tail = tail.join("\n");

                if !status.success() {
                    let code = status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "неизвестен".into());

                    return Err(CommandError::forge(format!(
                        "Установщик Forge завершился с кодом {code}"
                    ))
                    .with_details(tail));
                }

                return Ok(tail);
            }
        }
    }
}

async fn next_line<R>(lines: &mut Option<tokio::io::Lines<BufReader<R>>>) -> Option<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    match lines {
        Some(reader) => reader.next_line().await.ok().flatten(),
        None => std::future::pending().await,
    }
}

fn push_tail(tail: &mut Vec<String>, line: &str, on_log: &(impl Fn(&str) + Send + Sync)) {
    on_log(line);
    tail.push(line.to_string());
    if tail.len() > TAIL_LINES {
        tail.remove(0);
    }
}

struct InstallerOutput {
    client_jar: PathBuf,
    client_json: PathBuf,
}

async fn collect_output(workspace: &Path, forge_version: &str) -> Option<InstallerOutput> {
    let versions = workspace.join("versions");
    let mut entries = tokio::fs::read_dir(&versions).await.ok()?;

    let mut vanilla_jar: Option<PathBuf> = None;
    let mut forge_jar: Option<PathBuf> = None;
    let mut client_json: Option<PathBuf> = None;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let dir = entry.path();
        if !entry.file_type().await.map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }

        let Some(name) = dir.file_name().and_then(|name| name.to_str()) else { continue };

        let jar = dir.join(format!("{name}.jar"));
        let json = dir.join(format!("{name}.json"));
        let is_forge = mentions_forge(name, forge_version);

        if is_forge && json.is_file() {
            client_json = Some(json);
        }

        if jar.is_file() {
            if is_forge {
                forge_jar = Some(jar);
            } else {
                vanilla_jar = Some(jar);
            }
        }
    }

    Some(InstallerOutput {
        client_jar: vanilla_jar.or(forge_jar)?,
        client_json: client_json?,
    })
}

fn mentions_forge(dir_name: &str, forge_version: &str) -> bool {
    let name = dir_name.to_ascii_lowercase();

    if !name.contains("forge") {
        return false;
    }

    match forge_version.split_once('-') {
        Some((_, build)) => name.contains(&build.to_ascii_lowercase()),
        None => true,
    }
}

async fn link_libraries(shared: &Path, link: &Path) -> bool {
    let shared = shared.to_path_buf();
    let link = link.to_path_buf();

    tokio::task::spawn_blocking(move || {
        #[cfg(unix)]
        let result = std::os::unix::fs::symlink(&shared, &link);

        #[cfg(windows)]
        let result = std::os::windows::fs::symlink_dir(&shared, &link);

        match result {
            Ok(()) => true,
            Err(error) => {
                eprintln!(
                    "Не удалось связать каталог библиотек для установщика Forge,                      он скачает своё и результат будет перенесён: {error}"
                );
                false
            }
        }
    })
    .await
    .unwrap_or(false)
}

pub fn trim_log_line(line: &str) -> String {
    const LIMIT: usize = 90;

    let line = line.trim();
    if line.chars().count() <= LIMIT {
        return line.to_string();
    }

    let tail: String = line.chars().skip(line.chars().count() - LIMIT).collect();
    format!("…{tail}")
}

fn new_command(program: &str) -> Command {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_forge_profile_directories() {
        assert!(mentions_forge("1.20.1-forge-47.2.0", "1.20.1-47.2.0"));
        assert!(mentions_forge("1.12.2-forge1.12.2-14.23.5.2859", "1.12.2-14.23.5.2859"));
        assert!(!mentions_forge("1.20.1", "1.20.1-47.2.0"));
        assert!(!mentions_forge("1.20.1-forge-47.1.0", "1.20.1-47.2.0"));
    }

    #[test]
    fn long_log_lines_are_trimmed_from_the_left() {
        let line = "x".repeat(200);
        let trimmed = trim_log_line(&line);

        assert!(trimmed.starts_with('…'));
        assert_eq!(trimmed.chars().count(), 91);
        assert_eq!(trim_log_line("  короткая  "), "короткая");
    }

    #[test]
    fn tail_keeps_only_last_lines() {
        let mut tail = Vec::new();
        let sink = |_: &str| {};

        for index in 0..TAIL_LINES + 10 {
            push_tail(&mut tail, &format!("line {index}"), &sink);
        }

        assert_eq!(tail.len(), TAIL_LINES);
        assert_eq!(tail.last().unwrap(), &format!("line {}", TAIL_LINES + 9));
    }
}
