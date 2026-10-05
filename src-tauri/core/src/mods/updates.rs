use std::collections::BTreeMap;

use serde::Serialize;
use tokio::task::JoinSet;

use std::path::Path;

use crate::error::CommandResult;
use crate::instance::{LoaderType, PackProvider};
use crate::net::download::{DownloadOptions, DownloadRegistry, DownloadTask};

use super::catalog::{self, CatalogMatch, CatalogVersion};
use super::index::ModsIndex;
use super::{ModFile, ModsScan, DISABLED_SUFFIX, FOLDER};

const CONCURRENCY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdate {
    pub path: String,
    pub provider: PackProvider,
    pub project_id: String,
    pub title: String,
    pub from: String,
    pub to: String,
    pub version_id: String,
    pub file_name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    pub page_url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Updated {
    pub updated: Vec<String>,
    pub failed: Vec<String>,
}

pub fn candidates<'a>(
    mods: &'a [ModFile],
    matches: &'a BTreeMap<String, CatalogMatch>,
) -> Vec<(&'a ModFile, &'a CatalogMatch)> {
    mods.iter()
        .filter(|file| !file.managed)
        .filter_map(|file| matches.get(&file.path).map(|matched| (file, matched)))
        .collect()
}

pub async fn check(
    mods: &[ModFile],
    matches: &BTreeMap<String, CatalogMatch>,
    loader: LoaderType,
    minecraft_version: &str,
) -> Vec<ModUpdate> {
    let mut queue = candidates(mods, matches)
        .into_iter()
        .map(|(file, matched)| (file.path.clone(), matched.clone()))
        .collect::<Vec<_>>()
        .into_iter();

    let mut tasks = JoinSet::new();
    let mut updates = Vec::new();

    for _ in 0..CONCURRENCY {
        match queue.next() {
            Some(entry) => spawn(&mut tasks, entry, loader, minecraft_version),
            None => break,
        }
    }

    while let Some(joined) = tasks.join_next().await {
        if let Ok(Some(update)) = joined {
            updates.push(update);
        }

        if let Some(entry) = queue.next() {
            spawn(&mut tasks, entry, loader, minecraft_version);
        }
    }

    updates.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

    updates
}

fn spawn(
    tasks: &mut JoinSet<Option<ModUpdate>>,
    (path, matched): (String, CatalogMatch),
    loader: LoaderType,
    minecraft_version: &str,
) {
    let minecraft_version = minecraft_version.to_string();

    tasks.spawn(async move {
        let latest = latest(&matched, loader, &minecraft_version).await?;

        newer(&path, &matched, latest)
    });
}

async fn latest(
    matched: &CatalogMatch,
    loader: LoaderType,
    minecraft_version: &str,
) -> Option<CatalogVersion> {
    let found = match matched.provider {
        PackProvider::Modrinth => {
            let loaders: Vec<&str> = match loader {
                LoaderType::Vanilla => Vec::new(),
                other => vec![other.key()],
            };

            crate::modrinth::latest_mod(&matched.project_id, &loaders, minecraft_version).await
        }
        PackProvider::CurseForge => {
            crate::curseforge::latest_mod(&matched.project_id, loader, minecraft_version).await
        }
    };

    match found {
        Ok(version) => version,
        Err(error) => {
            log::warn!(
                "Failed to check for an update of '{}': {}",
                matched.title,
                error
            );
            None
        }
    }
}

fn newer(path: &str, matched: &CatalogMatch, latest: CatalogVersion) -> Option<ModUpdate> {
    if latest.version_id.trim() == matched.version_id.trim() || latest.url.trim().is_empty() {
        return None;
    }

    Some(ModUpdate {
        path: path.to_string(),
        provider: matched.provider,
        project_id: matched.project_id.clone(),
        title: matched.title.clone(),
        from: matched.version_number.clone(),
        to: latest.version_number,
        version_id: latest.version_id,
        file_name: latest.file_name,
        url: latest.url,
        sha1: latest.sha1,
        size: latest.size,
        page_url: matched.page_url.clone(),
    })
}

impl ModUpdate {
    fn matched(&self) -> CatalogMatch {
        CatalogMatch {
            provider: self.provider,
            project_id: self.project_id.clone(),
            version_id: self.version_id.clone(),
            version_number: self.to.clone(),
            title: self.title.clone(),
            slug: String::new(),
            icon_url: String::new(),
            page_url: self.page_url.clone(),
            authors: Vec::new(),
        }
    }
}

pub async fn apply(
    scan: &ModsScan,
    downloads: &DownloadRegistry,
    updates: &[ModUpdate],
    catalog_cache: &Path,
) -> CommandResult<Updated> {
    let mut report = Updated::default();

    if updates.is_empty() {
        return Ok(report);
    }

    crate::fs_util::ensure_dir(&scan.dir).await?;

    let mut index = ModsIndex::load(&scan.index_file).await;
    let mut forgotten = false;
    let mut remembered: Vec<(String, CatalogMatch)> = Vec::new();

    for (number, update) in updates.iter().enumerate() {
        let name = target_name(&update.file_name, &update.path);
        let destination = scan.dir.join(&name);

        let task = DownloadTask::verified(
            update.url.clone(),
            destination.clone(),
            update.size,
            update.sha1.clone(),
        );

        let downloaded = downloads
            .run(
                format!("mods-update:{number}:{}", update.path),
                vec![task],
                DownloadOptions::default(),
                None,
            )
            .await;

        if let Err(error) = downloaded {
            log::warn!("Failed to update '{}': {}", update.title, error);
            report.failed.push(update.title.clone());
            continue;
        }

        let previous = old_file_name(&update.path);

        if previous != name {
            let _ = tokio::fs::remove_file(scan.dir.join(&previous)).await;
        }

        forgotten |= index.forget(&update.path);
        forgotten |= index.forget(&format!("{FOLDER}/{name}"));

        if let Some(sha1) = &update.sha1 {
            remembered.push((sha1.to_lowercase(), update.matched()));
        }

        report.updated.push(update.title.clone());
    }

    if forgotten {
        let _ = index.save(&scan.index_file).await;
    }

    catalog::remember_all(catalog_cache, &remembered).await;

    Ok(report)
}

fn target_name(file_name: &str, old_path: &str) -> String {
    let name = crate::curseforge::pack::sanitize(file_name);

    match old_path.ends_with(DISABLED_SUFFIX) {
        true => format!("{name}{DISABLED_SUFFIX}"),
        false => name,
    }
}

fn old_file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::{ModDetails, ModKind};

    fn mod_file(path: &str, managed: bool) -> ModFile {
        ModFile {
            path: path.to_string(),
            file_name: old_file_name(path),
            enabled: !path.ends_with(DISABLED_SUFFIX),
            size: 10,
            modified: 1,
            kind: ModKind::Jar,
            managed,
            details: ModDetails::default(),
        }
    }

    fn matched(version_id: &str) -> CatalogMatch {
        CatalogMatch {
            provider: PackProvider::Modrinth,
            project_id: "AANobbMI".into(),
            version_id: version_id.into(),
            version_number: "0.5.3".into(),
            title: "Sodium".into(),
            slug: "sodium".into(),
            icon_url: String::new(),
            page_url: "https://modrinth.com/mod/sodium".into(),
            authors: Vec::new(),
        }
    }

    fn version(id: &str) -> CatalogVersion {
        CatalogVersion {
            version_id: id.into(),
            version_number: "0.6.0".into(),
            file_name: "sodium-0.6.0.jar".into(),
            url: "https://cdn.modrinth.com/sodium-0.6.0.jar".into(),
            sha1: Some("aaa".into()),
            size: Some(100),
            date: None,
            release: "release".into(),
            blocked: false,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn a_mod_from_a_pack_is_never_offered_an_update() {
        let mods = vec![
            mod_file("mods/sodium.jar", true),
            mod_file("mods/jei.jar", false),
        ];

        let matches = BTreeMap::from([
            ("mods/sodium.jar".to_string(), matched("old")),
            ("mods/jei.jar".to_string(), matched("old")),
        ]);

        let candidates = candidates(&mods, &matches);

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].0.path, "mods/jei.jar",
            "only our own mods are updated"
        );
    }

    #[test]
    fn a_mod_nobody_recognised_is_not_a_candidate() {
        let mods = vec![mod_file("mods/secret.jar", false)];

        assert!(candidates(&mods, &BTreeMap::new()).is_empty());
    }

    #[test]
    fn the_same_version_is_not_an_update() {
        assert!(newer("mods/sodium.jar", &matched("abc"), version("abc")).is_none());
        assert!(newer("mods/sodium.jar", &matched("abc"), version(" abc ")).is_none());
    }

    #[test]
    fn a_fresher_version_becomes_an_update() {
        let update = newer("mods/sodium.jar", &matched("abc"), version("xyz")).unwrap();

        assert_eq!(update.from, "0.5.3");
        assert_eq!(update.to, "0.6.0");
        assert_eq!(update.file_name, "sodium-0.6.0.jar");
        assert_eq!(update.page_url, "https://modrinth.com/mod/sodium");
    }

    #[test]
    fn a_version_without_a_file_is_not_an_update() {
        let mut broken = version("xyz");
        broken.url = String::new();

        assert!(newer("mods/sodium.jar", &matched("abc"), broken).is_none());
    }

    #[test]
    fn an_updated_mod_teaches_the_catalog_cache_its_new_version() {
        let update = newer("mods/sodium.jar", &matched("abc"), version("xyz")).unwrap();
        let entry = update.matched();

        assert_eq!(entry.version_id, "xyz");
        assert_eq!(
            entry.version_number, "0.6.0",
            "the cache already has the new version"
        );
        assert_eq!(entry.project_id, "AANobbMI");
    }

    #[test]
    fn a_switched_off_mod_stays_switched_off_after_the_update() {
        assert_eq!(
            target_name("sodium-0.6.0.jar", "mods/sodium-0.5.3.jar"),
            "sodium-0.6.0.jar"
        );
        assert_eq!(
            target_name("sodium-0.6.0.jar", "mods/sodium-0.5.3.jar.disabled"),
            "sodium-0.6.0.jar.disabled"
        );
    }

    #[test]
    fn a_file_name_from_the_catalog_cannot_walk_out_of_the_folder() {
        let name = target_name("../../evil.jar", "mods/old.jar");

        assert!(!name.contains('/'), "{name}");
        assert!(!name.contains('\\'), "{name}");
    }

    #[tokio::test]
    async fn nothing_to_update_touches_neither_disk_nor_network() {
        let scan = ModsScan {
            dir: std::path::PathBuf::from("/nope/mods"),
            index_file: std::path::PathBuf::from("/nope/mods-index.json"),
            icons: std::path::PathBuf::from("/nope/icons"),
            loader: None,
            managed: Default::default(),
        };

        let report = apply(
            &scan,
            &DownloadRegistry::new(),
            &[],
            std::path::Path::new("/nope/mod-catalog.json"),
        )
        .await
        .unwrap();

        assert_eq!(report, Updated::default());
    }

    #[tokio::test]
    async fn checking_without_candidates_asks_no_catalog() {
        let mods = vec![mod_file("mods/sodium.jar", true)];
        let matches = BTreeMap::from([("mods/sodium.jar".to_string(), matched("old"))]);

        assert!(check(&mods, &matches, LoaderType::Fabric, "1.20.1")
            .await
            .is_empty());
    }
}
