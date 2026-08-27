use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::error::CommandResult;
use crate::fs_util::{read_json_opt, write_json_atomic};
use crate::instance::PackProvider;

use super::hash::{self, FileHashes};
use super::index::ModsIndex;
use super::{ModFile, ModKind, ModsScan};

pub const VERSION: u32 = 1;

const MISSING_TTL: u64 = 14 * 24 * 60 * 60 * 1000;

const BATCH: usize = 500;

const HASH_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMatch {
    pub provider: PackProvider,
    pub project_id: String,
    pub version_id: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub page_url: String,
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogVersion {
    pub version_id: String,
    pub version_number: String,
    pub file_name: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found: Option<CatalogMatch>,
    pub checked_at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CatalogCache {
    pub version: u32,
    pub entries: BTreeMap<String, CacheEntry>,
}

impl CatalogCache {
    pub fn new() -> Self {
        Self {
            version: VERSION,
            entries: BTreeMap::new(),
        }
    }

    pub async fn load(path: &Path) -> Self {
        read_json_opt::<Self>(path)
            .await
            .filter(|cache| cache.version == VERSION)
            .unwrap_or_else(Self::new)
    }

    pub async fn save(&self, path: &Path) -> CommandResult<()> {
        write_json_atomic(path, self).await
    }

    pub fn lookup(&self, sha1: &str, now: u64) -> Option<&CacheEntry> {
        self.entries.get(sha1).filter(|entry| match entry.found {
            Some(_) => true,
            None => now.saturating_sub(entry.checked_at) < MISSING_TTL,
        })
    }

    pub fn remember(&mut self, sha1: &str, found: Option<CatalogMatch>, now: u64) {
        self.entries.insert(
            sha1.to_string(),
            CacheEntry {
                found,
                checked_at: now,
            },
        );
    }
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

pub async fn identify(
    scan: &ModsScan,
    mods: &[ModFile],
    cache_file: &Path,
) -> CommandResult<BTreeMap<String, CatalogMatch>> {
    let hashes = hashes_of(scan, mods).await?;

    if hashes.is_empty() {
        return Ok(BTreeMap::new());
    }

    let now = now_millis();
    let mut cache = CatalogCache::load(cache_file).await;

    let mut found: BTreeMap<String, CatalogMatch> = BTreeMap::new();
    let mut unknown: Vec<&FileHashes> = Vec::new();

    for (path, file) in &hashes {
        match cache.lookup(&file.sha1, now) {
            Some(entry) => {
                if let Some(matched) = &entry.found {
                    found.insert(path.clone(), matched.clone());
                }
            }
            None => unknown.push(file),
        }
    }

    if unknown.is_empty() {
        return Ok(found);
    }

    let mut matched: BTreeMap<String, CatalogMatch> = BTreeMap::new();

    let sha1s: Vec<String> = unknown.iter().map(|file| file.sha1.clone()).collect();

    for chunk in sha1s.chunks(BATCH) {
        match crate::modrinth::identify(chunk).await {
            Ok(page) => matched.extend(page),
            Err(error) => eprintln!("Modrinth не опознал моды: {}", error.message),
        }
    }

    if crate::curseforge::is_available() {
        let fingerprints: Vec<u32> = unknown
            .iter()
            .filter(|file| !matched.contains_key(&file.sha1))
            .filter_map(|file| file.fingerprint)
            .collect();

        for chunk in fingerprints.chunks(BATCH) {
            match crate::curseforge::identify(chunk).await {
                Ok(page) => {
                    for file in &unknown {
                        let Some(fingerprint) = file.fingerprint else { continue };
                        let Some(found) = page.get(&fingerprint) else { continue };

                        matched.insert(file.sha1.clone(), found.clone());
                    }
                }
                Err(error) => eprintln!("CurseForge не опознал моды: {}", error.message),
            }
        }
    }

    for file in &unknown {
        cache.remember(&file.sha1, matched.get(&file.sha1).cloned(), now);
    }

    if let Err(error) = cache.save(cache_file).await {
        eprintln!("Не удалось сохранить кэш каталога: {}", error.message);
    }

    for (path, file) in &hashes {
        if let Some(found_now) = matched.get(&file.sha1) {
            found.insert(path.clone(), found_now.clone());
        }
    }

    Ok(found)
}

async fn hashes_of(scan: &ModsScan, mods: &[ModFile]) -> CommandResult<BTreeMap<String, FileHashes>> {
    let mut index = ModsIndex::load(&scan.index_file).await;

    let mut ready: BTreeMap<String, FileHashes> = BTreeMap::new();
    let mut pending: Vec<&ModFile> = Vec::new();

    for file in mods {
        if file.kind == ModKind::Folder {
            continue;
        }

        match index.hashes(&file.path, file.size, file.modified) {
            Some(hashes) => {
                ready.insert(file.path.clone(), hashes.clone());
            }
            None => pending.push(file),
        }
    }

    if pending.is_empty() {
        return Ok(ready);
    }

    let mut queue = pending.into_iter();
    let mut tasks = JoinSet::new();

    for _ in 0..HASH_CONCURRENCY {
        match queue.next() {
            Some(file) => spawn_hash(&mut tasks, scan, file),
            None => break,
        }
    }

    let mut counted = false;

    while let Some(joined) = tasks.join_next().await {
        if let Ok((path, size, modified, Some(hashes))) = joined {
            index.remember_hashes(&path, size, modified, &hashes);
            ready.insert(path, hashes);
            counted = true;
        }

        if let Some(file) = queue.next() {
            spawn_hash(&mut tasks, scan, file);
        }
    }

    if counted {
        if let Err(error) = index.save(&scan.index_file).await {
            eprintln!("Не удалось сохранить хэши модов: {}", error.message);
        }
    }

    Ok(ready)
}

fn spawn_hash(
    tasks: &mut JoinSet<(String, u64, u64, Option<FileHashes>)>,
    scan: &ModsScan,
    file: &ModFile,
) {
    let path = scan.dir.join(&file.file_name);
    let key = file.path.clone();
    let size = file.size;
    let modified = file.modified;

    tasks.spawn(async move { (key, size, modified, hash::of(&path).await) });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matched() -> CatalogMatch {
        CatalogMatch {
            provider: PackProvider::Modrinth,
            project_id: "AANobbMI".into(),
            version_id: "xyz".into(),
            version_number: "0.5.3".into(),
            title: "Sodium".into(),
            slug: "sodium".into(),
            icon_url: String::new(),
            page_url: "https://modrinth.com/mod/sodium".into(),
            authors: Vec::new(),
        }
    }

    #[test]
    fn a_found_mod_stays_in_the_cache_forever() {
        let mut cache = CatalogCache::new();
        cache.remember("aaa", Some(matched()), 0);

        let entry = cache.lookup("aaa", MISSING_TTL * 10).expect("найденное не протухает");
        assert_eq!(entry.found.as_ref().unwrap().title, "Sodium");
    }

    #[test]
    fn a_missing_mod_is_asked_about_again_only_later() {
        let mut cache = CatalogCache::new();
        cache.remember("bbb", None, 1_000);

        assert!(cache.lookup("bbb", 1_000).is_some(), "только что спрашивали");
        assert!(cache.lookup("bbb", 1_000 + MISSING_TTL / 2).is_some());
        assert!(cache.lookup("bbb", 1_000 + MISSING_TTL).is_none(), "пора спросить снова");
    }

    #[test]
    fn an_unknown_hash_is_not_in_the_cache() {
        assert!(CatalogCache::new().lookup("ccc", 0).is_none());
    }

    #[tokio::test]
    async fn a_cache_from_an_older_format_is_ignored() {
        let dir = std::env::temp_dir().join(format!("cast-catalog-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mod-catalog.json");

        let mut cache = CatalogCache::new();
        cache.remember("aaa", Some(matched()), 0);
        cache.save(&file).await.unwrap();

        assert_eq!(CatalogCache::load(&file).await.entries.len(), 1);

        let stale = serde_json::json!({"version": VERSION + 1, "entries": {}});
        std::fs::write(&file, serde_json::to_vec(&stale).unwrap()).unwrap();

        assert!(CatalogCache::load(&file).await.entries.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
