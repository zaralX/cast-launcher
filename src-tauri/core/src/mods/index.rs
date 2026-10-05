use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CommandResult;
use crate::fs_util::{read_json_opt, write_json_atomic};

use super::hash::FileHashes;
use super::ModDetails;

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub size: u64,
    pub modified: u64,
    pub details: ModDetails,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hashes: Option<FileHashes>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModsIndex {
    pub version: u32,
    pub entries: BTreeMap<String, IndexEntry>,
}

impl ModsIndex {
    pub fn new() -> Self {
        Self {
            version: VERSION,
            entries: BTreeMap::new(),
        }
    }

    pub async fn load(path: &Path) -> Self {
        read_json_opt::<Self>(path)
            .await
            .filter(|index| index.version == VERSION)
            .unwrap_or_else(Self::new)
    }

    pub async fn save(&self, path: &Path) -> CommandResult<()> {
        write_json_atomic(path, self).await
    }

    pub fn reusable(&self, path: &str, size: u64, modified: u64) -> Option<&ModDetails> {
        self.entries
            .get(path)
            .filter(|entry| entry.size == size && entry.modified == modified)
            .map(|entry| &entry.details)
    }

    pub fn remember(&mut self, path: &str, size: u64, modified: u64, details: &ModDetails) {
        let hashes = self
            .entries
            .get(path)
            .filter(|entry| entry.size == size && entry.modified == modified)
            .and_then(|entry| entry.hashes.clone());

        self.entries.insert(
            path.to_string(),
            IndexEntry {
                size,
                modified,
                details: details.clone(),
                hashes,
            },
        );
    }

    pub fn hashes(&self, path: &str, size: u64, modified: u64) -> Option<&FileHashes> {
        self.entries
            .get(path)
            .filter(|entry| entry.size == size && entry.modified == modified)
            .and_then(|entry| entry.hashes.as_ref())
    }

    pub fn remember_hashes(
        &mut self,
        path: &str,
        size: u64,
        modified: u64,
        hashes: &FileHashes,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(path) else {
            return false;
        };

        if entry.size != size || entry.modified != modified {
            return false;
        }

        entry.hashes = Some(hashes.clone());

        true
    }

    pub fn rename(&mut self, from: &str, to: &str) -> bool {
        match self.entries.remove(from) {
            Some(entry) => self.entries.insert(to.to_string(), entry).is_none(),
            None => false,
        }
    }

    pub fn forget(&mut self, path: &str) -> bool {
        self.entries.remove(path).is_some()
    }

    pub fn retain(&mut self, alive: &[String]) -> bool {
        let before = self.entries.len();

        self.entries
            .retain(|path, _| alive.iter().any(|kept| kept == path));

        self.entries.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(name: &str) -> ModDetails {
        ModDetails {
            name: name.to_string(),
            ..ModDetails::default()
        }
    }

    #[test]
    fn an_untouched_file_is_taken_from_the_cache() {
        let mut index = ModsIndex::new();
        index.remember("mods/jei.jar", 100, 42, &details("JEI"));

        assert_eq!(index.reusable("mods/jei.jar", 100, 42).unwrap().name, "JEI");
        assert!(
            index.reusable("mods/jei.jar", 101, 42).is_none(),
            "размер изменился"
        );
        assert!(
            index.reusable("mods/jei.jar", 100, 43).is_none(),
            "файл перезаписан"
        );
        assert!(index.reusable("mods/other.jar", 100, 42).is_none());
    }

    #[test]
    fn deleted_mods_leave_the_cache() {
        let mut index = ModsIndex::new();
        index.remember("mods/jei.jar", 1, 1, &details("JEI"));
        index.remember("mods/gone.jar", 1, 1, &details("Gone"));

        assert!(index.retain(&["mods/jei.jar".to_string()]));
        assert!(
            !index.retain(&["mods/jei.jar".to_string()]),
            "второй раз выбрасывать нечего"
        );

        assert_eq!(index.entries.len(), 1);
        assert!(index.entries.contains_key("mods/jei.jar"));
    }

    #[test]
    fn hashes_live_next_to_the_metadata_and_survive_a_reparse() {
        let mut index = ModsIndex::new();
        index.remember("mods/jei.jar", 100, 42, &details("JEI"));

        let hashes = FileHashes {
            sha1: "aaa".into(),
            fingerprint: Some(7),
        };

        assert!(index.remember_hashes("mods/jei.jar", 100, 42, &hashes));
        assert_eq!(index.hashes("mods/jei.jar", 100, 42), Some(&hashes));

        index.remember("mods/jei.jar", 100, 42, &details("JEI 2"));
        assert_eq!(
            index.hashes("mods/jei.jar", 100, 42),
            Some(&hashes),
            "разбор не сбрасывает хэш"
        );

        index.remember("mods/jei.jar", 101, 42, &details("JEI 3"));
        assert!(
            index.hashes("mods/jei.jar", 101, 42).is_none(),
            "файл изменился - хэш заново"
        );
    }

    #[test]
    fn hashes_are_not_stored_for_a_file_the_index_does_not_know() {
        let mut index = ModsIndex::new();

        assert!(!index.remember_hashes("mods/ghost.jar", 1, 1, &FileHashes::default()));

        assert!(index.entries.is_empty());
    }

    #[test]
    fn a_renamed_file_keeps_its_metadata() {
        let mut index = ModsIndex::new();
        index.remember("mods/jei.jar", 100, 42, &details("JEI"));

        assert!(index.rename("mods/jei.jar", "mods/jei.jar.disabled"));
        assert_eq!(
            index
                .reusable("mods/jei.jar.disabled", 100, 42)
                .unwrap()
                .name,
            "JEI"
        );
        assert!(index.reusable("mods/jei.jar", 100, 42).is_none());

        assert!(!index.rename("mods/ghost.jar", "mods/ghost.jar.disabled"));
    }

    #[test]
    fn forgetting_reports_whether_there_was_anything_to_forget() {
        let mut index = ModsIndex::new();
        index.remember("mods/jei.jar", 1, 1, &details("JEI"));

        assert!(index.forget("mods/jei.jar"));
        assert!(!index.forget("mods/jei.jar"));
    }

    #[tokio::test]
    async fn a_cache_from_an_older_format_is_ignored() {
        let dir = std::env::temp_dir().join(format!("cast-mods-index-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mods-index.json");

        let mut index = ModsIndex::new();
        index.remember("mods/jei.jar", 1, 1, &details("JEI"));
        index.save(&file).await.unwrap();

        assert_eq!(ModsIndex::load(&file).await.entries.len(), 1);

        let stale = serde_json::json!({"version": VERSION + 1, "entries": {"mods/jei.jar": {"size": 1, "modified": 1, "details": {}}}});
        std::fs::write(&file, serde_json::to_vec(&stale).unwrap()).unwrap();

        assert!(ModsIndex::load(&file).await.entries.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_missing_cache_reads_as_empty() {
        let index = ModsIndex::load(Path::new("/nope/mods-index.json")).await;

        assert_eq!(index.version, VERSION);
        assert!(index.entries.is_empty());
    }
}
