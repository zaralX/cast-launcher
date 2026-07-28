use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::error::{CommandError, CommandResult};

pub const TAIL_LIMIT: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFile {
    pub name: String,
    pub size: u64,
    pub modified: u64,
}

pub async fn list(dir: &Path) -> CommandResult<Vec<LogFile>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог логов", dir, e))?;

    let mut files = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог логов", dir, e))?
    {
        let Ok(metadata) = entry.metadata().await else { continue };

        if !metadata.is_file() {
            continue;
        }

        files.push(LogFile {
            name: entry.file_name().to_string_lossy().to_string(),
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|since| since.as_millis() as u64)
                .unwrap_or(0),
        });
    }

    files.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));

    Ok(files)
}

pub fn resolve(dir: &Path, name: &str) -> CommandResult<PathBuf> {
    crate::fs_util::child_file(dir, name)
}

pub async fn read_tail(path: &Path, max_bytes: usize) -> CommandResult<String> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| CommandError::io(format!("Не удалось прочитать лог: {}", path.display()), path, e))?;

    let start = bytes.len().saturating_sub(max_bytes);
    let text = String::from_utf8_lossy(&bytes[start..]).into_owned();

    if start == 0 {
        return Ok(text);
    }

    Ok(match text.find('\n') {
        Some(newline) => text[newline + 1..].to_string(),
        None => text,
    })
}

pub async fn remove(path: &Path) -> CommandResult<()> {
    tokio::fs::remove_file(path)
        .await
        .map_err(|e| CommandError::io(format!("Не удалось удалить лог: {}", path.display()), path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-logs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn missing_directory_has_no_logs() {
        let logs = list(&std::env::temp_dir().join("cast-logs-нет-такого")).await.unwrap();
        assert!(logs.is_empty());
    }

    #[tokio::test]
    async fn logs_are_listed_newest_first_and_directories_are_skipped() {
        let dir = temp_dir();
        std::fs::create_dir(dir.join("natives")).unwrap();
        std::fs::write(dir.join("1.log"), b"first").unwrap();
        std::fs::write(dir.join("2.log"), b"second line").unwrap();

        filetime_older(&dir.join("1.log"));

        let logs = list(&dir).await.unwrap();

        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].name, "2.log");
        assert_eq!(logs[0].size, "second line".len() as u64);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_rejects_paths_outside_the_instance() {
        let dir = Path::new("/logs/abc");

        assert!(resolve(dir, "../../config.json").is_err());
        assert!(resolve(dir, "nested/1.log").is_err());
        assert!(resolve(dir, "").is_err());
        assert!(resolve(dir, "..").is_err());
        assert_eq!(resolve(dir, "1.log").unwrap(), dir.join("1.log"));
    }

    #[cfg(windows)]
    #[test]
    fn resolve_rejects_windows_separators_and_drives() {
        let dir = Path::new("C:\\logs\\abc");

        assert!(resolve(dir, "..\\..\\config.json").is_err());
        assert!(resolve(dir, "C:\\windows\\system.ini").is_err());
    }

    #[tokio::test]
    async fn tail_keeps_the_end_of_the_file_and_drops_the_partial_line() {
        let dir = temp_dir();
        let file = dir.join("big.log");
        std::fs::write(&file, "первая строка\nвторая строка\nтретья строка\n").unwrap();

        let tail = read_tail(&file, 30).await.unwrap();

        assert!(tail.ends_with("третья строка\n"));
        assert!(!tail.contains("первая"));
        assert!(tail.starts_with("вторая") || tail.starts_with("третья"));

        let whole = read_tail(&file, TAIL_LIMIT).await.unwrap();
        assert!(whole.starts_with("первая строка"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn reading_a_missing_log_is_an_error() {
        let dir = temp_dir();
        assert!(read_tail(&dir.join("нет.log"), TAIL_LIMIT).await.is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    fn filetime_older(path: &Path) {
        let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        let earlier = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        file.set_modified(earlier).unwrap();
    }
}
