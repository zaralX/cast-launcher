use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{CommandError, CommandResult};

pub async fn ensure_dir(dir: &Path) -> CommandResult<()> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| CommandError::io(format!("Не удалось создать каталог: {}", dir.display()), dir, e))
}

pub fn child_file(dir: &Path, name: &str) -> CommandResult<PathBuf> {
    Path::new(name)
        .file_name()
        .filter(|file| *file == name && name != "." && name != "..")
        .map(|file| dir.join(file))
        .ok_or_else(|| CommandError::fs(format!("Недопустимое имя файла: {name}")))
}

pub async fn read_text(path: &Path) -> CommandResult<String> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| CommandError::io(format!("Не удалось прочитать файл: {}", path.display()), path, e))
}

pub async fn read_json_opt<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = tokio::fs::read_to_string(path).await.ok()?;

    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            eprintln!(
                "Повреждённый JSON, файл будет перезаписан: {} ({error})",
                path.display()
            );
            None
        }
    }
}

pub async fn read_json<T: DeserializeOwned>(path: &Path) -> CommandResult<T> {
    let text = read_text(path).await?;

    serde_json::from_str(&text).map_err(|e| {
        CommandError::manifest(format!("Повреждённый JSON: {}", path.display()))
            .with_details(e.to_string())
    })
}

pub async fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> CommandResult<()> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| CommandError::unknown("Не удалось сериализовать данные").with_details(e.to_string()))?;

    write_atomic(path, &bytes).await
}

pub async fn write_atomic(path: &Path, bytes: &[u8]) -> CommandResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }

    let temp = temp_sibling(path);

    tokio::fs::write(&temp, bytes)
        .await
        .map_err(|e| CommandError::io(format!("Не удалось записать файл: {}", path.display()), &temp, e))?;

    if let Err(error) = tokio::fs::rename(&temp, path).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(CommandError::io(
            format!("Не удалось сохранить файл: {}", path.display()),
            path,
            error,
        ));
    }

    Ok(())
}

pub async fn merge_dir(from: &Path, to: &Path) -> CommandResult<()> {
    if !from.is_dir() {
        return Ok(());
    }

    ensure_dir(to).await?;

    let mut entries = tokio::fs::read_dir(from)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог", from, e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог", from, e))?
    {
        let source = entry.path();
        let target = to.join(entry.file_name());

        let is_dir = entry
            .file_type()
            .await
            .map(|kind| kind.is_dir())
            .unwrap_or(false);

        if is_dir {
            Box::pin(merge_dir(&source, &target)).await?;
            continue;
        }

        if target.exists() {
            continue;
        }

        if let Some(parent) = target.parent() {
            ensure_dir(parent).await?;
        }

        tokio::fs::copy(&source, &target)
            .await
            .map_err(|e| CommandError::io("Не удалось перенести файл", &source, e))?;
    }

    Ok(())
}

pub async fn remove_file_if_exists(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

pub async fn remove_dir_if_exists(path: &Path) {
    let _ = tokio::fs::remove_dir_all(path).await;
}

fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let unique = uuid::Uuid::new_v4().simple().to_string();
    path.with_file_name(format!(".{name}.{unique}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn atomic_write_replaces_existing_file() {
        let dir = std::env::temp_dir().join(format!("cast-fs-{}", uuid::Uuid::new_v4()));
        let file = dir.join("nested").join("config.json");

        write_json_atomic(&file, &serde_json::json!({ "a": 1 })).await.unwrap();
        write_json_atomic(&file, &serde_json::json!({ "a": 2 })).await.unwrap();

        let value: serde_json::Value = read_json(&file).await.unwrap();
        assert_eq!(value["a"], 2);

        let leftovers = std::fs::read_dir(file.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn merge_keeps_existing_files_and_adds_new_ones() {
        let root = std::env::temp_dir().join(format!("cast-merge-{}", uuid::Uuid::new_v4()));
        let from = root.join("from");
        let to = root.join("to");

        std::fs::create_dir_all(from.join("nested")).unwrap();
        std::fs::create_dir_all(to.join("nested")).unwrap();

        std::fs::write(from.join("nested").join("shared.jar"), b"new").unwrap();
        std::fs::write(to.join("nested").join("shared.jar"), b"old").unwrap();
        std::fs::write(from.join("nested").join("fresh.jar"), b"fresh").unwrap();

        merge_dir(&from, &to).await.unwrap();

        assert_eq!(std::fs::read(to.join("nested").join("shared.jar")).unwrap(), b"old");
        assert_eq!(std::fs::read(to.join("nested").join("fresh.jar")).unwrap(), b"fresh");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn merging_a_missing_directory_is_not_an_error() {
        let root = std::env::temp_dir().join(format!("cast-merge-{}", uuid::Uuid::new_v4()));
        merge_dir(&root.join("nope"), &root.join("to")).await.unwrap();
    }

    #[test]
    fn child_file_stays_inside_the_directory() {
        let dir = Path::new("/data/icons");

        assert!(child_file(dir, "../../config.json").is_err());
        assert!(child_file(dir, "nested/icon.png").is_err());
        assert!(child_file(dir, "").is_err());
        assert!(child_file(dir, "..").is_err());
        assert_eq!(child_file(dir, "icon.png").unwrap(), dir.join("icon.png"));
    }

    #[tokio::test]
    async fn broken_json_reads_as_none() {
        let dir = std::env::temp_dir().join(format!("cast-fs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("broken.json");
        std::fs::write(&file, "{ not json").unwrap();

        let value: Option<serde_json::Value> = read_json_opt(&file).await;
        assert!(value.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }
}
