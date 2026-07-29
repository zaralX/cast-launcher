use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{ensure_dir, write_atomic};
use crate::net::http;
use crate::paths::LauncherPaths;

use super::{https_url, Catalog, Manifest};

pub const TIMEOUT: Duration = Duration::from_secs(8);

pub const MAX_SIZE: u64 = 1024 * 1024;

pub fn cache_dir(paths: &LauncherPaths) -> PathBuf {
    paths.cache().join("castpack")
}

pub fn catalog_cache(paths: &LauncherPaths) -> PathBuf {
    cache_dir(paths).join("catalog.json")
}

pub async fn catalog(url: &str, cache: &Path) -> CommandResult<Catalog> {
    match fetch(url).await {
        Ok(bytes) => {
            let catalog = Catalog::parse(&bytes)?;

            store(cache, &bytes).await;

            Ok(catalog)
        }
        Err(error) => match tokio::fs::read(cache).await {
            Ok(bytes) => {
                eprintln!("Каталог CastPack взят из кэша: {error}");
                Catalog::parse(&bytes)
            }
            Err(_) => Err(error),
        },
    }
}

pub async fn manifest(url: &str) -> CommandResult<Manifest> {
    Manifest::parse(&fetch(url).await?)
}

pub async fn installed_manifest(path: &Path) -> Option<Manifest> {
    let bytes = tokio::fs::read(path).await.ok()?;

    match Manifest::parse(&bytes) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
            eprintln!("Сохранённый манифест сборки не читается: {}", error.message);
            None
        }
    }
}

pub async fn save_manifest(path: &Path, manifest: &Manifest) -> CommandResult<()> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| {
        CommandError::unknown("Не удалось сохранить манифест сборки").with_details(e.to_string())
    })?;

    write_atomic(path, &bytes).await
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbedFile {
    pub file_name: String,
    pub sha1: String,
    pub size: u64,
}

pub async fn probe(url: &str) -> CommandResult<ProbedFile> {
    use sha1::{Digest, Sha1};

    let url = https_url(url)?;

    let response = http::client().get(url).send().await.map_err(|e| {
        CommandError::network(format!("Не удалось скачать {url}")).with_details(e.to_string())
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(http::http_status_error(status, url));
    }

    let bytes = response.bytes().await.map_err(|e| {
        CommandError::download(format!("Обрыв загрузки: {url}")).with_details(e.to_string())
    })?;

    Ok(ProbedFile {
        file_name: file_name_of(url),
        sha1: Sha1::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect(),
        size: bytes.len() as u64,
    })
}

pub fn file_name_of(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|path| path.rsplit('/').next())
        .unwrap_or_default()
        .to_string()
}

async fn fetch(url: &str) -> CommandResult<Vec<u8>> {
    let url = https_url(url)?;

    let response = http::client()
        .get(url)
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            CommandError::network(format!("Не удалось получить данные CastPack: {url}"))
                .with_details(e.to_string())
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(http::http_status_error(status, url));
    }

    if response.content_length().is_some_and(|size| size > MAX_SIZE) {
        return Err(CommandError::download(format!("Файл CastPack слишком большой: {url}")));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| {
            CommandError::network(format!("Обрыв загрузки CastPack: {url}")).with_details(e.to_string())
        })?
        .to_vec();

    if bytes.len() as u64 > MAX_SIZE {
        return Err(CommandError::download(format!("Файл CastPack слишком большой: {url}")));
    }

    Ok(bytes)
}

async fn store(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        if ensure_dir(parent).await.is_err() {
            return;
        }
    }

    if let Err(error) = write_atomic(path, bytes).await {
        eprintln!("Не удалось сохранить кэш каталога CastPack: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-castpack-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn catalog_json() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schemaVersion": super::super::SCHEMA_VERSION,
            "packs": [{
                "id": "zaralx-rpg",
                "name": "zaralX RPG",
                "version": "1.4.2",
                "manifest": "https://cdn.zaralx.ru/packs/rpg/manifest.json"
            }]
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn a_dead_network_falls_back_to_the_cached_catalog() {
        let dir = temp_dir();
        let cache = dir.join("catalog.json");

        std::fs::write(&cache, catalog_json()).unwrap();

        let loaded = catalog("https://такого.адреса.нет.invalid/catalog.json", &cache)
            .await
            .unwrap();

        assert_eq!(loaded.packs[0].id, "zaralx-rpg");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn without_a_cache_a_dead_network_is_an_error() {
        let dir = temp_dir();

        let loaded = catalog("https://такого.адреса.нет.invalid/catalog.json", &dir.join("нет.json")).await;

        assert!(loaded.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn plain_http_is_refused_before_any_request() {
        let dir = temp_dir();

        assert!(manifest("http://cdn.zaralx.ru/m.json").await.is_err());
        assert!(catalog("http://cdn.zaralx.ru/c.json", &dir.join("нет.json")).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_installed_manifest_survives_a_round_trip() {
        let dir = temp_dir();
        let path = dir.join("castpack.json");

        let manifest = Manifest::parse(
            &serde_json::to_vec(&json!({
                "schemaVersion": super::super::SCHEMA_VERSION,
                "id": "zaralx-rpg",
                "name": "zaralX RPG",
                "version": "1.4.2",
                "minecraft": "1.20.1"
            }))
            .unwrap(),
        )
        .unwrap();

        save_manifest(&path, &manifest).await.unwrap();

        assert_eq!(installed_manifest(&path).await.unwrap(), manifest);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_broken_saved_manifest_reads_as_nothing_installed() {
        let dir = temp_dir();
        let path = dir.join("castpack.json");

        std::fs::write(&path, "{ not json").unwrap();

        assert!(installed_manifest(&path).await.is_none());
        assert!(installed_manifest(&dir.join("нет.json")).await.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_file_name_comes_from_the_link_without_its_query() {
        assert_eq!(file_name_of("https://cdn.zaralx.ru/mods/jei-1.0.jar"), "jei-1.0.jar");
        assert_eq!(file_name_of("https://cdn.zaralx.ru/mods/jei.jar?v=2"), "jei.jar");
        assert_eq!(file_name_of("https://cdn.zaralx.ru/"), "");
    }

    #[tokio::test]
    async fn probing_a_non_https_link_never_leaves_the_launcher() {
        assert!(probe("http://cdn.zaralx.ru/a.jar").await.is_err());
    }

    #[test]
    fn the_cache_lives_next_to_the_other_launcher_caches() {
        let paths = LauncherPaths::new(PathBuf::from("/cfg"), None);

        assert!(catalog_cache(&paths).starts_with(paths.cache()));
        assert!(catalog_cache(&paths).ends_with("catalog.json"));
    }
}
