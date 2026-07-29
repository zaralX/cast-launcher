use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CommandResult;
use crate::fs_util::{read_json_opt, remove_file_if_exists, safe_join, write_json_atomic};
use crate::packs::BlockedFile;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PackFiles {
    pub version_id: String,
    pub paths: BTreeSet<String>,
}

impl PackFiles {
    pub fn new(version_id: impl Into<String>, paths: BTreeSet<String>) -> Self {
        Self {
            version_id: version_id.into(),
            paths,
        }
    }

    pub async fn load(path: &Path) -> Self {
        read_json_opt(path).await.unwrap_or_default()
    }

    pub async fn save(&self, path: &Path) -> CommandResult<()> {
        write_json_atomic(path, self).await
    }

    pub fn stale(&self, current: &BTreeSet<String>) -> Vec<String> {
        self.paths.difference(current).cloned().collect()
    }
}

pub async fn save_blocked(path: &Path, blocked: &[BlockedFile]) -> CommandResult<()> {
    if blocked.is_empty() {
        remove_file_if_exists(path).await;
        return Ok(());
    }

    write_json_atomic(path, &blocked.to_vec()).await
}

pub async fn load_blocked(path: &Path) -> Vec<BlockedFile> {
    read_json_opt(path).await.unwrap_or_default()
}

pub async fn remove(minecraft_dir: &Path, paths: &[String]) -> usize {
    let mut removed = 0;

    for relative in paths {
        let Ok(path) = safe_join(minecraft_dir, relative) else { continue };

        if tokio::fs::remove_file(&path).await.is_ok() {
            removed += 1;
            prune_empty_dirs(minecraft_dir, &path).await;
        }
    }

    removed
}

async fn prune_empty_dirs(root: &Path, file: &Path) {
    let mut current = file.parent().map(Path::to_path_buf);

    while let Some(dir) = current {
        if dir == root || !dir.starts_with(root) {
            break;
        }

        if tokio::fs::remove_dir(&dir).await.is_err() {
            break;
        }

        current = dir.parent().map(Path::to_path_buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    #[test]
    fn only_files_missing_from_the_new_version_are_stale() {
        let previous = PackFiles::new("v1", set(&["mods/a.jar", "mods/b.jar", "config/a.toml"]));
        let current = set(&["mods/a.jar", "mods/c.jar", "config/a.toml"]);

        assert_eq!(previous.stale(&current), vec!["mods/b.jar"]);
    }

    #[test]
    fn a_first_install_has_nothing_to_clean_up() {
        assert!(PackFiles::default().stale(&set(&["mods/a.jar"])).is_empty());
    }

    #[tokio::test]
    async fn a_missing_or_broken_record_is_treated_as_empty() {
        let dir = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        assert_eq!(PackFiles::load(&dir.join("нет.json")).await, PackFiles::default());

        let broken = dir.join("broken.json");
        std::fs::write(&broken, "{ not json").unwrap();
        assert_eq!(PackFiles::load(&broken).await, PackFiles::default());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn removing_stale_files_takes_their_empty_directories_with_them() {
        let root = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let minecraft = root.join("minecraft");

        std::fs::create_dir_all(minecraft.join("mods")).unwrap();
        std::fs::create_dir_all(minecraft.join("config").join("nested")).unwrap();

        std::fs::write(minecraft.join("mods").join("old.jar"), b"old").unwrap();
        std::fs::write(minecraft.join("mods").join("kept.jar"), b"kept").unwrap();
        std::fs::write(minecraft.join("config").join("nested").join("a.toml"), b"a").unwrap();

        let removed = remove(&minecraft, &set(&["mods/old.jar", "config/nested/a.toml"]).into_iter().collect::<Vec<_>>()).await;

        assert_eq!(removed, 2);
        assert!(minecraft.join("mods").join("kept.jar").is_file());
        assert!(minecraft.join("mods").is_dir(), "каталог с чужими файлами остаётся");
        assert!(!minecraft.join("config").exists(), "опустевшая ветка убирается целиком");
        assert!(minecraft.is_dir(), "сам каталог игры не трогаем");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn paths_cannot_reach_outside_the_instance() {
        let root = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let minecraft = root.join("minecraft");

        std::fs::create_dir_all(&minecraft).unwrap();
        std::fs::write(root.join("instance.json"), b"keep").unwrap();

        assert_eq!(remove(&minecraft, &["../instance.json".to_string()]).await, 0);
        assert!(root.join("instance.json").is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_blocked_list_is_cleared_once_nothing_is_blocked() {
        let dir = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let file = dir.join("pack-blocked.json");

        let blocked = vec![BlockedFile {
            file_name: "entityculling.jar".into(),
            target_path: "mods/entityculling.jar".into(),
            website_url: "https://www.curseforge.com/minecraft/mc-mods/entityculling/download/8287120".into(),
            sha1: Some("62ac7ed3bbc0b920428bcfc18d1962836b84c391".into()),
            local_path: None,
        }];

        save_blocked(&file, &blocked).await.unwrap();
        assert_eq!(load_blocked(&file).await, blocked);

        save_blocked(&file, &[]).await.unwrap();
        assert!(load_blocked(&file).await.is_empty());
        assert!(!file.exists(), "пустой список не должен оставлять файл");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn the_record_survives_a_round_trip() {
        let dir = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let file = dir.join("pack-files.json");

        let record = PackFiles::new("abc", set(&["mods/a.jar", "options.txt"]));
        record.save(&file).await.unwrap();

        assert_eq!(PackFiles::load(&file).await, record);

        std::fs::remove_dir_all(&dir).ok();
    }
}
