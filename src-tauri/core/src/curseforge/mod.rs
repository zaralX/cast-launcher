pub mod pack;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{CommandError, CommandResult};
use crate::instance::{LoaderType, PackProvider};
use crate::net::http;
use crate::net::meta_cache::MetaCache;
use crate::packs::{
    Category, FileHashes, PackFile, PackFilters, PackHit, PackPage, PackVersion, SearchQuery,
};

pub const API: &str = "https://api.curseforge.com/v1";

pub const GAME_ID: u32 = 432;

pub const MODPACK_CLASS: u32 = 4471;

pub const WORLD_CLASS: u32 = 17;

pub const RESOURCE_PACK_CLASS: u32 = 12;

pub const SHADER_CLASS: u32 = 6552;

pub const DATA_PACK_CLASS: u32 = 6945;

pub const MAX_LIMIT: u32 = 50;
pub const MAX_OFFSET: u32 = 10_000;

const FILES_PAGE: u32 = 10_000;

pub const ICON_HOSTS: &[&str] = &["forgecdn.net"];

const BUNDLED_KEY: &str = "$2a$10$wuAJuNZuted3NORVmpgUC.m8sI.pv1tOPKZyBgLFGjxFp/br0lZCC";

pub fn api_key() -> &'static str {
    match option_env!("CAST_CURSEFORGE_API_KEY") {
        Some(key) if !key.is_empty() => key,
        _ => BUNDLED_KEY,
    }
}

pub fn is_available() -> bool {
    !api_key().trim().is_empty()
}

fn headers() -> [(&'static str, &'static str); 2] {
    [("x-api-key", api_key()), ("Accept", "application/json")]
}

/// https://docs.curseforge.com/?python#tocS_ModsSearchSortField
fn sort_field(sort: &str) -> u32 {
    match sort {
        "downloads" => 6, // TotalDownloads
        "updated" => 3,   // LastUpdated
        _ => 1,           // Featured
    }
}

/// https://docs.curseforge.com/?http#tocS_ModLoaderType
fn loader_id(name: &str) -> Option<u32> {
    match name.trim().to_ascii_lowercase().as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

pub fn search_url(query: &SearchQuery) -> String {
    let mut url = Url::parse(&format!("{API}/mods/search")).expect("постоянный адрес поиска CurseForge");

    let limit = query.limit.clamp(1, MAX_LIMIT);
    let offset = query.offset.min(MAX_OFFSET.saturating_sub(limit));

    {
        let mut pairs = url.query_pairs_mut();

        pairs.append_pair("gameId", &GAME_ID.to_string());
        pairs.append_pair("classId", &MODPACK_CLASS.to_string());
        pairs.append_pair("index", &offset.to_string());
        pairs.append_pair("pageSize", &limit.to_string());

        let text = query.query.trim();
        if !text.is_empty() {
            pairs.append_pair("searchFilter", text);
        }

        pairs.append_pair("sortField", &sort_field(query.sort_key()).to_string());
        pairs.append_pair("sortOrder", "desc");

        let loaders: Vec<String> = SearchQuery::clean(&query.loaders)
            .filter_map(loader_id)
            .map(|id| id.to_string())
            .collect();

        if !loaders.is_empty() {
            pairs.append_pair("modLoaderTypes", &format!("[{}]", loaders.join(",")));
        }

        let categories: Vec<&str> = SearchQuery::clean(&query.categories)
            .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
            .collect();

        if !categories.is_empty() {
            pairs.append_pair("categoryIds", &format!("[{}]", categories.join(",")));
        }

        if let Some(version) = SearchQuery::clean(&query.game_versions).next() {
            pairs.append_pair("gameVersion", version);
        }
    }

    url.into()
}

pub async fn search(query: &SearchQuery) -> CommandResult<PackPage> {
    let limit = query.limit.clamp(1, MAX_LIMIT);
    let offset = query.offset.min(MAX_OFFSET.saturating_sub(limit));

    let page: Envelope<Vec<RawMod>> = get_json(&search_url(query)).await?;
    let total = page.pagination.map(|p| p.total_count).unwrap_or(0);

    Ok(PackPage {
        hits: page.data.into_iter().map(PackHit::from).collect(),
        offset,
        limit,
        total_hits: total.min(MAX_OFFSET),
    })
}

pub async fn versions(project_id: &str) -> CommandResult<Vec<PackVersion>> {
    let id = numeric(project_id)?;
    let mut collected: Vec<RawFile> = Vec::new();
    let mut index = 0;

    loop {
        let url = format!("{API}/mods/{id}/files?index={index}&pageSize={FILES_PAGE}");
        let page: Envelope<Vec<RawFile>> = get_json(&url).await?;

        let received = page.data.len() as u32;
        collected.extend(page.data);

        let total = page.pagination.map(|p| p.total_count).unwrap_or(0);
        index += received;

        if received == 0 || index >= total {
            break;
        }
    }

    collected.sort_by(|a, b| b.file_date.cmp(&a.file_date));

    Ok(collected.into_iter().map(PackVersion::from).collect())
}

pub async fn version(project_id: &str, version_id: &str) -> CommandResult<PackVersion> {
    let project = numeric(project_id)?;
    let file = numeric(version_id)?;

    let raw: Envelope<RawFile> = get_json(&format!("{API}/mods/{project}/files/{file}")).await?;

    Ok(PackVersion::from(raw.data))
}

pub async fn download_page(project_id: &str, version_id: &str) -> CommandResult<String> {
    let project = numeric(project_id)?;
    let file = numeric(version_id)?;

    let raw: Envelope<RawMod> = get_json(&format!("{API}/mods/{project}")).await?;

    Ok(raw
        .data
        .website_url()
        .map(|url| format!("{}/download/{file}", url.trim_end_matches('/')))
        .unwrap_or_default())
}

pub async fn filters(meta: &MetaCache) -> CommandResult<PackFilters> {
    let categories_url = format!("{API}/categories?gameId={GAME_ID}&classId={MODPACK_CLASS}");
    let headers = headers();

    let (categories, manifest) = tokio::try_join!(
        meta.fetch_json_with::<Envelope<Vec<RawCategory>>>(&categories_url, &headers),
        crate::meta::vanilla::manifest(meta),
    )?;

    let mut categories: Vec<Category> = categories
        .data
        .into_iter()
        .map(|category| Category {
            id: category.id.to_string(),
            label: category.name,
            header: "categories".into(),
        })
        .collect();

    categories.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));

    Ok(PackFilters {
        categories,
        loaders: vec!["fabric".into(), "forge".into(), "neoforge".into()],
        game_versions: manifest
            .versions
            .into_iter()
            .filter(|version| version.release_type.as_deref() == Some("release"))
            .map(|version| version.id)
            .collect(),
    })
}

pub async fn icon(url: &str) -> CommandResult<Vec<u8>> {
    crate::packs::fetch_icon(url, ICON_HOSTS).await
}

fn numeric(value: &str) -> CommandResult<&str> {
    let value = value.trim();

    let valid = !value.is_empty()
        && value.len() <= 12
        && value.chars().all(|symbol| symbol.is_ascii_digit());

    valid
        .then_some(value)
        .ok_or_else(|| CommandError::manifest(format!("Недопустимый идентификатор CurseForge: {value}")))
}

#[derive(Debug, Clone, Deserialize)]
struct Envelope<T> {
    data: T,
    #[serde(default)]
    pagination: Option<Pagination>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pagination {
    #[serde(default)]
    total_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCategory {
    id: u64,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLogo {
    #[serde(default)]
    url: String,
    #[serde(default)]
    thumbnail_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAuthor {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLinks {
    #[serde(default)]
    website_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFileIndex {
    #[serde(default)]
    game_version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawMod {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    date_modified: Option<String>,
    #[serde(default)]
    logo: Option<RawLogo>,
    #[serde(default)]
    authors: Vec<RawAuthor>,
    #[serde(default)]
    categories: Vec<RawCategory>,
    #[serde(default)]
    links: Option<RawLinks>,
    #[serde(default)]
    class_id: Option<u32>,
    #[serde(default)]
    allow_mod_distribution: Option<bool>,
    #[serde(default)]
    latest_files_indexes: Vec<RawFileIndex>,
}

impl RawMod {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn website_url(&self) -> Option<&str> {
        self.links.as_ref().and_then(|links| links.website_url.as_deref())
    }

    pub(crate) fn target_folder(&self) -> &'static str {
        match self.class_id {
            Some(WORLD_CLASS) => "saves",
            Some(RESOURCE_PACK_CLASS) => "resourcepacks",
            Some(SHADER_CLASS) => "shaderpacks",
            Some(DATA_PACK_CLASS) => "datapacks",
            _ => "mods",
        }
    }
}

impl From<RawMod> for PackHit {
    fn from(raw: RawMod) -> Self {
        let icon_url = raw.logo.as_ref().and_then(|logo| {
            let url = match logo.thumbnail_url.is_empty() {
                true => &logo.url,
                false => &logo.thumbnail_url,
            };

            (!url.is_empty()).then(|| url.clone())
        });

        let categories: Vec<String> = raw.categories.iter().map(|c| c.name.clone()).collect();

        let mut versions: Vec<String> = raw
            .latest_files_indexes
            .iter()
            .map(|index| index.game_version.clone())
            .filter(|version| version.contains('.'))
            .collect();

        versions.sort();
        versions.dedup();

        Self {
            provider: PackProvider::CurseForge,
            project_id: raw.id.to_string(),
            slug: raw.slug,
            title: raw.name,
            description: raw.summary,
            icon_url,
            author: raw.authors.first().map(|author| author.name.clone()),
            downloads: raw.download_count.max(0.0) as u64,
            follows: 0,
            display_categories: categories.clone(),
            categories,
            versions,
            client_side: None,
            server_side: None,
            date_modified: raw.date_modified,
            website_url: raw.links.and_then(|links| links.website_url),
            distribution_allowed: raw.allow_mod_distribution.unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawHash {
    #[serde(default)]
    value: String,
    #[serde(default)]
    algo: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawFile {
    id: u64,
    #[serde(default)]
    mod_id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    release_type: u32,
    #[serde(default)]
    file_date: Option<String>,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    file_length: Option<u64>,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    hashes: Vec<RawHash>,
}

impl RawFile {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn mod_id(&self) -> u64 {
        self.mod_id
    }

    pub(crate) fn file_name(&self) -> &str {
        &self.file_name
    }

    pub(crate) fn download_url(&self) -> Option<&str> {
        self.download_url.as_deref().filter(|url| !url.is_empty())
    }

    pub(crate) fn size(&self) -> Option<u64> {
        self.file_length
    }

    pub(crate) fn sha1(&self) -> Option<String> {
        self.hashes
            .iter()
            .find(|hash| hash.algo == 1 && !hash.value.is_empty())
            .map(|hash| hash.value.clone())
    }
}

fn split_game_versions(values: &[String]) -> (Vec<String>, Vec<String>) {
    let mut minecraft = Vec::new();
    let mut loaders = Vec::new();

    for value in values {
        let value = value.trim();

        if value.is_empty() {
            continue;
        }

        if value.contains('.') {
            minecraft.push(value.to_string());
        } else if matches!(
            value.to_ascii_lowercase().as_str(),
            "forge" | "neoforge" | "fabric" | "quilt" | "cauldron" | "liteloader"
        ) {
            loaders.push(value.to_ascii_lowercase());
        }
    }

    (minecraft, loaders)
}

pub(crate) fn loader_from_tags(loaders: &[String]) -> Option<LoaderType> {
    loaders.iter().find_map(|loader| match loader.trim().to_ascii_lowercase().as_str() {
        "fabric" => Some(LoaderType::Fabric),
        "forge" => Some(LoaderType::Forge),
        "neoforge" => Some(LoaderType::NeoForge),
        _ => None,
    })
}

fn release_type(value: u32) -> &'static str {
    match value {
        2 => "beta",
        3 => "alpha",
        _ => "release",
    }
}

impl From<RawFile> for PackVersion {
    fn from(raw: RawFile) -> Self {
        let (game_versions, loaders) = split_game_versions(&raw.game_versions);

        let loader = loader_from_tags(&loaders);
        let minecraft_version = game_versions.last().cloned();
        let blocked = raw.download_url().is_none();

        let file = (!raw.file_name.is_empty()).then(|| PackFile {
            url: raw.download_url().unwrap_or_default().to_string(),
            filename: raw.file_name.clone(),
            size: raw.file_length,
            hashes: FileHashes {
                sha1: raw.sha1(),
                sha512: None,
            },
        });

        Self {
            provider: PackProvider::CurseForge,
            supported: loader.is_some() && minecraft_version.is_some() && file.is_some(),
            id: raw.id.to_string(),
            project_id: raw.mod_id.to_string(),
            name: raw.display_name.clone(),
            version_number: raw.display_name,
            version_type: release_type(raw.release_type).to_string(),
            downloads: raw.download_count.max(0.0) as u64,
            date_published: raw.file_date,
            game_versions,
            loaders,
            minecraft_version,
            loader,
            file,
            blocked,
        }
    }
}

pub(crate) async fn get_json<T: DeserializeOwned>(url: &str) -> CommandResult<T> {
    let mut request = http::client().get(url);

    for (name, value) in headers() {
        request = request.header(name, value);
    }

    let response = request.send().await.map_err(|e| {
        CommandError::network("Не удалось связаться с CurseForge").with_details(format!("{url}\n{e}"))
    })?;

    read_json(response, url).await
}

pub(crate) async fn post_json<B: Serialize, T: DeserializeOwned>(
    url: &str,
    body: &B,
) -> CommandResult<T> {
    let mut request = http::client().post(url).json(body);

    for (name, value) in headers() {
        request = request.header(name, value);
    }

    let response = request.send().await.map_err(|e| {
        CommandError::network("Не удалось связаться с CurseForge").with_details(format!("{url}\n{e}"))
    })?;

    read_json(response, url).await
}

async fn read_json<T: DeserializeOwned>(response: reqwest::Response, url: &str) -> CommandResult<T> {
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let mut error = http::http_status_error(status, url);

        if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::UNAUTHORIZED {
            error = CommandError::network(
                "CurseForge отклонил ключ API - лаунчер собран со старым или недействительным ключом",
            );
        }

        if let Some(details) = api_error(&body) {
            return Err(error.with_details(details));
        }

        return Err(error);
    }

    let body = response.bytes().await.map_err(|e| {
        CommandError::network(format!("Обрыв ответа CurseForge: {url}")).with_details(e.to_string())
    })?;

    serde_json::from_slice(&body).map_err(|e| {
        CommandError::manifest("CurseForge ответил в неожиданном формате")
            .with_details(format!("{url}\n{e}"))
    })
}

fn api_error(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;

    if let Some(message) = value.get("message").and_then(serde_json::Value::as_str) {
        return Some(message.to_string());
    }

    let errors = value.get("errors")?.as_object()?;

    let text = errors
        .iter()
        .map(|(field, messages)| {
            let list = messages
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            format!("{field}: {list}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::SearchQuery;

    fn query() -> SearchQuery {
        SearchQuery {
            provider: PackProvider::CurseForge,
            ..Default::default()
        }
    }

    #[test]
    fn search_is_always_limited_to_minecraft_modpacks() {
        let url = search_url(&query());

        assert!(url.starts_with("https://api.curseforge.com/v1/mods/search?"));
        assert!(url.contains(&format!("gameId={GAME_ID}")));
        assert!(url.contains(&format!("classId={MODPACK_CLASS}")));
        assert!(url.contains("sortOrder=desc"));
    }

    #[test]
    fn the_page_is_clamped_to_what_the_api_accepts() {
        let url = search_url(&SearchQuery {
            limit: 900,
            offset: 9_999,
            ..query()
        });

        assert!(url.contains(&format!("pageSize={MAX_LIMIT}")));
        assert!(url.contains("index=9950"));

        let zero = search_url(&SearchQuery {
            limit: 0,
            ..query()
        });
        assert!(zero.contains("pageSize=1"));
    }

    #[test]
    fn filters_are_translated_into_curseforge_numbers() {
        let url = search_url(&SearchQuery {
            loaders: vec!["fabric".into(), "neoforge".into()],
            categories: vec!["4481".into()],
            ..query()
        });

        assert!(url.contains("modLoaderTypes=%5B4%2C6%5D"), "{url}");
        assert!(url.contains("categoryIds=%5B4481%5D"), "{url}");
    }

    #[test]
    fn junk_filter_values_never_reach_the_api() {
        let url = search_url(&SearchQuery {
            loaders: vec!["  ".into(), "babric".into()],
            categories: vec!["4481; drop".into(), String::new()],
            ..query()
        });

        assert!(!url.contains("modLoaderTypes"), "неизвестный загрузчик отбрасываем");
        assert!(!url.contains("categoryIds"), "нечисловую категорию отбрасываем");
    }

    #[test]
    fn only_the_first_game_version_is_sent() {
        let url = search_url(&SearchQuery {
            game_versions: vec!["1.20.1".into(), "1.21".into()],
            ..query()
        });

        assert!(url.contains("gameVersion=1.20.1"));
        assert_eq!(url.matches("gameVersion=").count(), 1, "API принимает только одну");
    }

    #[test]
    fn the_query_is_encoded_and_sorts_are_mapped() {
        let url = search_url(&SearchQuery {
            query: "  всё лучшее сразу ".into(),
            sort: Some("downloads".into()),
            ..query()
        });

        assert!(!url.contains(' '));
        assert!(url.contains("sortField=6"));

        let follows = search_url(&SearchQuery {
            sort: Some("follows".into()),
            ..query()
        });
        assert!(follows.contains("sortField=1"));
    }

    #[test]
    fn identifiers_from_the_front_cannot_walk_the_api_path() {
        assert!(numeric("../../admin").is_err());
        assert!(numeric("925200x").is_err());
        assert!(numeric("").is_err());
        assert_eq!(numeric(" 925200 ").unwrap(), "925200");
    }

    fn file(json: serde_json::Value) -> PackVersion {
        PackVersion::from(serde_json::from_value::<RawFile>(json).unwrap())
    }

    #[test]
    fn game_versions_are_split_into_minecraft_and_loaders() {
        let version = file(serde_json::json!({
            "id": 8469481,
            "modId": 925200,
            "displayName": "All the Mods 10-7.2",
            "fileName": "All the Mods 10-7.2.zip",
            "releaseType": 1,
            "fileDate": "2026-07-20T04:47:46.327Z",
            "fileLength": 200194596u64,
            "downloadUrl": "https://edge.forgecdn.net/files/8469/481/pack.zip",
            "gameVersions": ["1.21.1", "NeoForge", "Client"],
            "hashes": [{"value": "aa09", "algo": 1}, {"value": "f1f8", "algo": 2}]
        }));

        assert_eq!(version.minecraft_version.as_deref(), Some("1.21.1"));
        assert_eq!(version.loader, Some(LoaderType::NeoForge));
        assert_eq!(version.game_versions, vec!["1.21.1"]);
        assert_eq!(version.loaders, vec!["neoforge"]);
        assert!(version.supported);
        assert!(!version.blocked);

        let archive = version.file.unwrap();
        assert_eq!(archive.hashes.sha1.as_deref(), Some("aa09"), "md5 нам не подходит");
        assert_eq!(archive.size, Some(200194596));
    }

    #[test]
    fn a_pack_without_a_link_is_still_installable_by_hand() {
        let version = file(serde_json::json!({
            "id": 4635891,
            "modId": 886999,
            "displayName": "QoLCraft",
            "fileName": "QoLCraft.zip",
            "releaseType": 1,
            "fileLength": 4096,
            "gameVersions": ["1.20.1", "Forge"],
            "hashes": [{"value": "abc", "algo": 1}],
            "downloadUrl": null
        }));

        assert!(version.blocked, "ссылки нет - качать будет пользователь");
        assert!(
            version.supported,
            "отказывать в установке нельзя: архив можно скачать вручную"
        );
        assert!(version.unsupported_reason().is_none());

        let archive = version.file.unwrap();
        assert!(archive.url.is_empty());
        assert_eq!(archive.filename, "QoLCraft.zip");
        assert_eq!(archive.hashes.sha1.as_deref(), Some("abc"));
        assert_eq!(archive.size, Some(4096));
    }

    #[test]
    fn a_version_without_even_a_file_name_is_not_installable() {
        let version = file(serde_json::json!({
            "id": 1, "modId": 2, "displayName": "x", "releaseType": 1,
            "gameVersions": ["1.20.1", "Forge"]
        }));

        assert!(version.file.is_none());
        assert!(!version.supported);
    }

    #[test]
    fn unsupported_loaders_are_not_offered_for_install() {
        let quilt = file(serde_json::json!({
            "id": 1, "modId": 2, "displayName": "x", "fileName": "x.zip", "releaseType": 1,
            "gameVersions": ["1.20.1", "Quilt"],
            "downloadUrl": "https://edge.forgecdn.net/files/1/1/x.zip"
        }));

        assert!(quilt.loader.is_none());
        assert!(!quilt.supported);
        assert!(quilt.unsupported_reason().unwrap().contains("quilt"));
    }

    #[test]
    fn release_types_are_named() {
        let kind = |value: u32| {
            file(serde_json::json!({
                "id": 1, "modId": 2, "displayName": "x", "fileName": "x.zip",
                "releaseType": value, "gameVersions": ["1.20.1", "Forge"],
                "downloadUrl": "https://edge.forgecdn.net/files/1/1/x.zip"
            }))
            .version_type
        };

        assert_eq!(kind(1), "release");
        assert_eq!(kind(2), "beta");
        assert_eq!(kind(3), "alpha");
        assert_eq!(kind(99), "release", "незнакомое считаем релизом");
    }

    #[test]
    fn a_hit_carries_over_what_the_card_shows() {
        let raw: RawMod = serde_json::from_value(serde_json::json!({
            "id": 925200,
            "name": "All the Mods 10 - ATM10",
            "slug": "all-the-mods-10",
            "summary": "Всё сразу",
            "downloadCount": 20482494.0,
            "dateModified": "2026-07-20T04:47:46.327Z",
            "logo": {"url": "https://media.forgecdn.net/a.png", "thumbnailUrl": "https://media.forgecdn.net/thumb.png"},
            "authors": [{"name": "ATMTeam"}, {"name": "oly2o6"}],
            "categories": [{"id": 4482, "name": "Extra Large"}],
            "links": {"websiteUrl": "https://www.curseforge.com/minecraft/modpacks/all-the-mods-10"},
            "classId": 4471,
            "allowModDistribution": true,
            "latestFilesIndexes": [{"gameVersion": "1.21.1"}, {"gameVersion": "1.21.1"}, {"gameVersion": "NeoForge"}]
        }))
        .unwrap();

        let hit = PackHit::from(raw);

        assert_eq!(hit.project_id, "925200");
        assert_eq!(hit.downloads, 20_482_494);
        assert_eq!(hit.author.as_deref(), Some("ATMTeam"));
        assert_eq!(hit.icon_url.as_deref(), Some("https://media.forgecdn.net/thumb.png"));
        assert_eq!(hit.versions, vec!["1.21.1"], "дубли и загрузчики в список версий не идут");
        assert_eq!(hit.follows, 0);
        assert!(hit.distribution_allowed);
    }

    #[test]
    fn a_pack_that_forbids_third_party_downloads_is_flagged() {
        let raw: RawMod = serde_json::from_value(serde_json::json!({
            "id": 1, "name": "x", "slug": "x", "allowModDistribution": false
        }))
        .unwrap();

        assert!(!PackHit::from(raw).distribution_allowed);

        let unknown: RawMod = serde_json::from_value(serde_json::json!({"id": 1})).unwrap();
        assert!(
            PackHit::from(unknown).distribution_allowed,
            "молчание считаем разрешением, как и Prism"
        );
    }

    #[test]
    fn every_kind_of_project_lands_in_its_own_folder() {
        let folder = |class_id: serde_json::Value| {
            serde_json::from_value::<RawMod>(serde_json::json!({"id": 1, "classId": class_id}))
                .unwrap()
                .target_folder()
        };

        assert_eq!(folder(serde_json::json!(17)), "saves");
        assert_eq!(folder(serde_json::json!(12)), "resourcepacks");
        assert_eq!(folder(serde_json::json!(6552)), "shaderpacks");
        assert_eq!(folder(serde_json::json!(6945)), "datapacks");
        assert_eq!(folder(serde_json::json!(6)), "mods");
        assert_eq!(folder(serde_json::json!(4546)), "mods", "оформление кладём к модам");
        assert_eq!(folder(serde_json::Value::Null), "mods");
    }

    #[test]
    fn both_error_shapes_are_turned_into_readable_text() {
        assert_eq!(
            api_error(r#"{"message":"Index + PageSize cannot exceed 10000"}"#).unwrap(),
            "Index + PageSize cannot exceed 10000"
        );

        let validation = api_error(
            r#"{"errors":{"PageSize":["The field PageSize must be between 1 and 50."]},"status":400}"#,
        )
        .unwrap();
        assert!(validation.contains("PageSize"));
        assert!(validation.contains("between 1 and 50"));

        assert!(api_error("не json").is_none());
    }

    #[test]
    fn the_launcher_ships_with_a_usable_key() {
        assert!(is_available(), "без ключа источник CurseForge не включится");
    }

    #[tokio::test]
    #[ignore = "ходит в сеть"]
    async fn the_catalog_answers_the_way_we_parse_it() {
        let page = search(&SearchQuery {
            query: "All the Mods".into(),
            limit: 5,
            ..query()
        })
        .await
        .unwrap();

        assert!(!page.hits.is_empty(), "по такому запросу что-то обязано найтись");
        assert!(page.total_hits > 0);

        let hit = &page.hits[0];
        assert_eq!(hit.provider, PackProvider::CurseForge);
        assert!(!hit.title.is_empty());
        assert!(hit.downloads > 0, "счётчик загрузок приходит дробным числом");
        assert!(hit.icon_url.as_deref().is_some_and(|url| url.contains("forgecdn.net")));
    }

    #[tokio::test]
    #[ignore = "ходит в сеть"]
    async fn versions_of_a_real_pack_are_installable() {
        let versions = versions("925200").await.unwrap();

        assert!(versions.len() > 10, "у пака должна быть история версий");
        assert!(versions.iter().any(|version| version.supported));

        let newest = &versions[0];
        assert!(newest.minecraft_version.is_some());
        assert!(newest.date_published.is_some());

        assert!(versions.windows(2).all(|pair| pair[0].date_published >= pair[1].date_published));
    }

    #[tokio::test]
    #[ignore = "ходит в сеть"]
    async fn a_blocked_pack_still_offers_everything_needed_to_fetch_it_by_hand() {
        let version = version("886999", "4635891").await.unwrap();

        assert!(version.blocked);
        assert!(version.supported, "ставится вручную");

        let archive = version.file.unwrap();
        assert!(archive.url.is_empty());
        assert!(!archive.filename.is_empty());
        assert!(archive.hashes.sha1.is_some(), "без хеша нечем проверить скачанное");

        let page = download_page("886999", "4635891").await.unwrap();
        assert!(page.starts_with("https://www.curseforge.com/"), "{page}");
        assert!(page.ends_with("/download/4635891"), "{page}");
    }
}
