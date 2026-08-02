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
    pub seeded: BTreeSet<String>,
    pub extracted: BTreeSet<String>,
}

impl PackFiles {
    pub fn new(version_id: impl Into<String>, paths: BTreeSet<String>) -> Self {
        Self {
            version_id: version_id.into(),
            paths,
            seeded: BTreeSet::new(),
            extracted: BTreeSet::new(),
        }
    }

    pub fn with_seeded(mut self, seeded: BTreeSet<String>) -> Self {
        self.seeded = seeded;
        self
    }

    pub fn with_extracted(mut self, extracted: BTreeSet<String>) -> Self {
        self.extracted = extracted;
        self
    }

    pub async fn load(path: &Path) -> Self {
        read_json_opt(path).await.unwrap_or_default()
    }

    pub async fn save(&self, path: &Path) -> CommandResult<()> {
        write_json_atomic(path, self).await
    }

    pub fn stale(&self, current: &BTreeSet<String>) -> Vec<String> {
        self.paths
            .union(&self.extracted)
            .filter(|path| !current.contains(*path))
            .cloned()
            .collect()
    }

    pub async fn missing(&self, minecraft_dir: &Path) -> Vec<String> {
        let mut missing = Vec::new();

        for relative in &self.paths {
            let Ok(path) = safe_join(minecraft_dir, relative) else { continue };

            if !path.is_file() {
                missing.push(relative.clone());
            }
        }

        missing
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
    fn files_from_overrides_are_cleaned_up_just_like_downloaded_ones() {
        let previous = PackFiles::new("v1", set(&["mods/a.jar"]))
            .with_extracted(set(&["config/a.toml", "kubejs/x.js"]));

        assert_eq!(previous.stale(&set(&["mods/a.jar", "kubejs/x.js"])), vec!["config/a.toml"]);
    }

    #[tokio::test]
    async fn a_file_the_game_rewrote_is_not_a_reason_to_reinstall() {
        let root = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let minecraft = root.join("minecraft");

        std::fs::create_dir_all(minecraft.join("mods")).unwrap();
        std::fs::write(minecraft.join("mods").join("a.jar"), b"a").unwrap();

        let record = PackFiles::new("v1", set(&["mods/a.jar"]))
            .with_extracted(set(&["config/euphoria_patcher/data.json"]));

        assert!(
            record.missing(&minecraft).await.is_empty(),
            "файлы из overrides мод вправе переименовать или убрать"
        );

        std::fs::remove_dir_all(&root).ok();
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

        let record = PackFiles::new("abc", set(&["mods/a.jar", "options.txt"]))
            .with_seeded(set(&["servers.dat"]));
        record.save(&file).await.unwrap();

        assert_eq!(PackFiles::load(&file).await, record);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn records_written_before_seeding_existed_read_back_empty() {
        let dir = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let file = dir.join("pack-files.json");

        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, r#"{"versionId":"abc","paths":["mods/a.jar"]}"#).unwrap();

        let record = PackFiles::load(&file).await;

        assert_eq!(record.version_id, "abc");
        assert!(record.seeded.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_file_deleted_by_hand_is_reported_as_missing() {
        let root = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let minecraft = root.join("minecraft");

        std::fs::create_dir_all(minecraft.join("mods")).unwrap();
        std::fs::write(minecraft.join("mods").join("kept.jar"), b"kept").unwrap();

        let record = PackFiles::new("v1", set(&["mods/kept.jar", "mods/gone.jar"]))
            .with_seeded(set(&["options.txt"]));

        assert_eq!(record.missing(&minecraft).await, vec!["mods/gone.jar"]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_seeded_file_the_player_removed_is_not_a_reason_to_reinstall() {
        let root = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let minecraft = root.join("minecraft");

        std::fs::create_dir_all(&minecraft).unwrap();

        let record = PackFiles::new("v1", BTreeSet::new()).with_seeded(set(&["options.txt"]));

        assert!(record.missing(&minecraft).await.is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_folder_in_place_of_a_file_counts_as_missing() {
        let root = std::env::temp_dir().join(format!("cast-pack-{}", uuid::Uuid::new_v4()));
        let minecraft = root.join("minecraft");

        std::fs::create_dir_all(minecraft.join("mods").join("a.jar")).unwrap();

        let record = PackFiles::new("v1", set(&["mods/a.jar"]));

        assert_eq!(record.missing(&minecraft).await, vec!["mods/a.jar"]);

        std::fs::remove_dir_all(&root).ok();
    }
}
