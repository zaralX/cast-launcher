use std::path::Path;
use std::time::{Duration, SystemTime};

use sha1::{Digest, Sha1};

use crate::error::CommandResult;
use crate::fs_util::child_file;
use crate::icons;

const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

pub fn key(path: &str, size: u64, modified: u64, extension: &str) -> String {
    let digest = Sha1::digest(format!("{path}|{size}|{modified}").as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    format!("{hex}.{extension}")
}

pub fn store(dir: &Path, key: &str, bytes: &[u8]) -> CommandResult<()> {
    let path = child_file(dir, key)?;

    std::fs::create_dir_all(dir).ok();

    std::fs::write(&path, bytes)
        .map_err(|e| crate::error::CommandError::io("error.reason.fs.write_file", &path, e))
}

pub fn exists(dir: &Path, key: &str) -> bool {
    child_file(dir, key)
        .map(|path| path.is_file())
        .unwrap_or(false)
}

pub async fn data_url(dir: &Path, key: &str) -> CommandResult<String> {
    icons::data_url(&icons::resolve(dir, key)?).await
}

pub async fn prune(dir: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(metadata) = entry.metadata().await else {
            continue;
        };

        let stale = metadata
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .map(|age| age > MAX_AGE)
            .unwrap_or(false);

        if metadata.is_file() && stale {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_follows_the_file_it_came_from() {
        let first = key("mods/jei.jar", 100, 42, "png");

        assert_eq!(
            first,
            key("mods/jei.jar", 100, 42, "png"),
            "same file, same key"
        );
        assert_ne!(
            first,
            key("mods/jei.jar", 100, 43, "png"),
            "the mod was updated"
        );
        assert_ne!(first, key("mods/other.jar", 100, 42, "png"));
        assert!(first.ends_with(".png"));
    }

    #[test]
    fn a_key_never_leaves_the_cache_directory() {
        let dir = std::env::temp_dir().join(format!("cast-mod-icons-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        assert!(store(&dir, "../escape.png", b"x").is_err());
        assert!(store(&dir, "nested/icon.png", b"x").is_err());
        assert!(store(&dir, "icon.png", b"x").is_ok());
        assert!(exists(&dir, "icon.png"));
        assert!(!exists(&dir, "../escape.png"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn pruning_a_missing_directory_is_not_an_error() {
        prune(Path::new("/nope/mod-icons")).await;
    }

    #[tokio::test]
    async fn fresh_icons_survive_pruning() {
        let dir = std::env::temp_dir().join(format!("cast-mod-icons-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        store(&dir, "icon.png", b"\x89PNG\r\n\x1a\n").unwrap();

        prune(&dir).await;

        assert!(exists(&dir, "icon.png"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
