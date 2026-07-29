pub mod pack;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{CommandError, CommandResult};
use crate::instance::LoaderType;
use crate::net::http;
use crate::net::meta_cache::MetaCache;

pub const API: &str = "https://api.modrinth.com/v2";

pub const MAX_LIMIT: u32 = 100;

const SORTS: &[&str] = &["relevance", "downloads", "follows", "newest", "updated"];

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchQuery {
    pub query: String,
    pub categories: Vec<String>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
    pub environment: Option<String>,
    pub sort: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

impl SearchQuery {
    pub fn url(&self) -> String {
        let mut url = Url::parse(&format!("{API}/search")).expect("постоянный адрес поиска Modrinth");

        {
            let mut pairs = url.query_pairs_mut();

            let query = self.query.trim();
            if !query.is_empty() {
                pairs.append_pair("query", query);
            }

            pairs.append_pair("facets", &self.facets());
            pairs.append_pair("index", self.sort());
            pairs.append_pair("offset", &self.offset.to_string());
            pairs.append_pair("limit", &self.limit().to_string());
        }

        url.into()
    }

    /// Группы фасетов складываются по И, значения внутри группы — по ИЛИ.
    /// Как на самом Modrinth: загрузчики и версии игры — ИЛИ, категории — И.
    fn facets(&self) -> String {
        let mut groups: Vec<Vec<String>> = vec![vec!["project_type:modpack".into()]];

        let loaders: Vec<String> = clean(&self.loaders)
            .map(|loader| format!("categories:{loader}"))
            .collect();
        if !loaders.is_empty() {
            groups.push(loaders);
        }

        for category in clean(&self.categories) {
            groups.push(vec![format!("categories:{category}")]);
        }

        let versions: Vec<String> = clean(&self.game_versions)
            .map(|version| format!("versions:{version}"))
            .collect();
        if !versions.is_empty() {
            groups.push(versions);
        }

        match self.environment.as_deref().map(str::trim) {
            Some("client") => groups.push(vec!["client_side:required".into()]),
            Some("server") => groups.push(vec!["server_side:required".into()]),
            _ => {}
        }

        serde_json::to_string(&groups).unwrap_or_else(|_| r#"[["project_type:modpack"]]"#.into())
    }

    fn sort(&self) -> &str {
        self.sort
            .as_deref()
            .map(str::trim)
            .filter(|sort| SORTS.contains(sort))
            .unwrap_or("relevance")
    }

    fn limit(&self) -> u32 {
        self.limit.clamp(1, MAX_LIMIT)
    }
}

fn clean(values: &[String]) -> impl Iterator<Item = &str> {
    values.iter().map(|value| value.trim()).filter(|value| !value.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SearchHit {
    pub project_id: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub display_categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub client_side: Option<String>,
    #[serde(default)]
    pub server_side: Option<String>,
    #[serde(default)]
    pub date_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SearchPage {
    #[serde(default)]
    pub hits: Vec<SearchHit>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Version {
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub date_published: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<VersionFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct VersionFile {
    pub url: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub hashes: FileHashes,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct FileHashes {
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
}

impl Version {
    pub fn pack_file(&self) -> Option<&VersionFile> {
        let mrpack = |file: &&VersionFile| file.filename.to_ascii_lowercase().ends_with(".mrpack");

        self.files
            .iter()
            .find(|file| file.primary && mrpack(file))
            .or_else(|| self.files.iter().find(mrpack))
    }

    pub fn loader(&self) -> Option<LoaderType> {
        loader_from_tags(&self.loaders)
    }

    pub fn minecraft_version(&self) -> Option<&str> {
        self.game_versions.last().map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: String,
    pub downloads: u64,
    pub date_published: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub minecraft_version: Option<String>,
    pub loader: Option<LoaderType>,
    pub file: Option<VersionFile>,
    pub supported: bool,
}

impl From<Version> for VersionSummary {
    fn from(version: Version) -> Self {
        let loader = version.loader();
        let minecraft_version = version.minecraft_version().map(str::to_string);
        let file = version.pack_file().cloned();

        Self {
            supported: loader.is_some() && minecraft_version.is_some() && file.is_some(),
            id: version.id,
            project_id: version.project_id,
            name: version.name,
            version_number: version.version_number,
            version_type: version.version_type,
            downloads: version.downloads,
            date_published: version.date_published,
            game_versions: version.game_versions,
            loaders: version.loaders,
            minecraft_version,
            loader,
            file,
        }
    }
}

pub fn loader_from_tags(loaders: &[String]) -> Option<LoaderType> {
    loaders.iter().find_map(|loader| match loader.trim().to_ascii_lowercase().as_str() {
        "fabric" => Some(LoaderType::Fabric),
        "forge" => Some(LoaderType::Forge),
        "neoforge" => Some(LoaderType::NeoForge),
        "minecraft" | "vanilla" => Some(LoaderType::Vanilla),
        _ => None,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Filters {
    pub categories: Vec<Category>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub name: String,
    pub header: String,
}

pub async fn search(query: &SearchQuery) -> CommandResult<SearchPage> {
    get_json(&query.url()).await
}

pub async fn versions(project_id: &str) -> CommandResult<Vec<VersionSummary>> {
    let id = segment(project_id)?;
    let raw: Vec<Version> = get_json(&format!("{API}/project/{id}/version")).await?;

    Ok(raw.into_iter().map(VersionSummary::from).collect())
}

pub async fn version(version_id: &str) -> CommandResult<VersionSummary> {
    let id = segment(version_id)?;
    let raw: Version = get_json(&format!("{API}/version/{id}")).await?;

    Ok(VersionSummary::from(raw))
}

pub async fn filters(meta: &MetaCache) -> CommandResult<Filters> {
    #[derive(Deserialize)]
    struct RawCategory {
        name: String,
        #[serde(default)]
        header: String,
        #[serde(default)]
        project_type: String,
    }

    #[derive(Deserialize)]
    struct RawLoader {
        name: String,
        #[serde(default)]
        supported_project_types: Vec<String>,
    }

    #[derive(Deserialize)]
    struct RawGameVersion {
        version: String,
        #[serde(default)]
        version_type: String,
    }

    let (categories_url, loaders_url, versions_url) = (
        format!("{API}/tag/category"),
        format!("{API}/tag/loader"),
        format!("{API}/tag/game_version"),
    );

    let (raw_categories, raw_loaders, raw_versions) = tokio::try_join!(
        meta.fetch_json::<Vec<RawCategory>>(&categories_url),
        meta.fetch_json::<Vec<RawLoader>>(&loaders_url),
        meta.fetch_json::<Vec<RawGameVersion>>(&versions_url),
    )?;

    let categories = raw_categories
        .into_iter()
        .filter(|category| category.project_type == "modpack")
        .map(|category| Category {
            name: category.name,
            header: category.header,
        })
        .collect();

    let loaders = raw_loaders
        .into_iter()
        .filter(|loader| loader.supported_project_types.iter().any(|kind| kind == "modpack"))
        .map(|loader| loader.name)
        .filter(|name| loader_from_tags(std::slice::from_ref(name)).is_some())
        .collect();

    let game_versions = raw_versions
        .into_iter()
        .filter(|version| version.version_type == "release")
        .map(|version| version.version)
        .collect();

    Ok(Filters {
        categories,
        loaders,
        game_versions,
    })
}

pub const ICON_HOST: &str = "cdn.modrinth.com";

pub async fn icon(url: &str) -> CommandResult<Vec<u8>> {
    let parsed = Url::parse(url)
        .map_err(|e| CommandError::network("Некорректная ссылка на иконку").with_details(e.to_string()))?;

    if parsed.scheme() != "https" || parsed.host_str() != Some(ICON_HOST) {
        return Err(CommandError::network(format!("Иконка не из каталога {ICON_HOST}: {url}")));
    }

    let response = http::client().get(parsed.as_str()).send().await.map_err(|e| {
        CommandError::network("Не удалось скачать иконку").with_details(format!("{url}\n{e}"))
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(http::http_status_error(status, url));
    }

    if response.content_length().is_some_and(|size| size > crate::icons::MAX_SIZE) {
        return Err(CommandError::download(format!("Иконка слишком большая: {url}")));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| CommandError::download(format!("Обрыв загрузки иконки: {url}")).with_details(e.to_string()))?;

    if bytes.len() as u64 > crate::icons::MAX_SIZE {
        return Err(CommandError::download(format!("Иконка слишком большая: {url}")));
    }

    Ok(bytes.to_vec())
}

pub fn icon_file_name(project_id: &str, url: &str) -> String {
    let extension = url
        .split('?')
        .next()
        .and_then(|path| path.rsplit('/').next())
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .filter(|extension| {
            (1..=5).contains(&extension.len()) && extension.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or_else(|| "png".to_string());

    format!("modrinth-{project_id}.{extension}")
}

fn segment(value: &str) -> CommandResult<&str> {
    let value = value.trim();

    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|symbol| symbol.is_ascii_alphanumeric() || matches!(symbol, '-' | '_' | '.'));

    valid
        .then_some(value)
        .ok_or_else(|| CommandError::manifest(format!("Недопустимый идентификатор Modrinth: {value}")))
}

async fn get_json<T: DeserializeOwned>(url: &str) -> CommandResult<T> {
    let response = http::client().get(url).send().await.map_err(|e| {
        CommandError::network("Не удалось связаться с Modrinth").with_details(format!("{url}\n{e}"))
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(http::http_status_error(status, url));
    }

    response.json::<T>().await.map_err(|e| {
        CommandError::manifest("Modrinth ответил в неожиданном формате").with_details(format!("{url}\n{e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_facets(query: &SearchQuery) -> Vec<Vec<String>> {
        serde_json::from_str(&query.facets()).unwrap()
    }

    #[test]
    fn search_is_always_limited_to_modpacks() {
        let facets = parse_facets(&SearchQuery::default());
        assert_eq!(facets, vec![vec!["project_type:modpack"]]);
    }

    #[test]
    fn loaders_and_versions_are_or_categories_are_and() {
        let query = SearchQuery {
            loaders: vec!["fabric".into(), "forge".into()],
            categories: vec!["adventure".into(), "magic".into()],
            game_versions: vec!["1.20.1".into(), "1.21".into()],
            ..Default::default()
        };

        let facets = parse_facets(&query);

        assert!(facets.contains(&vec!["categories:fabric".to_string(), "categories:forge".to_string()]));
        assert!(facets.contains(&vec!["categories:adventure".to_string()]));
        assert!(facets.contains(&vec!["categories:magic".to_string()]));
        assert!(facets.contains(&vec!["versions:1.20.1".to_string(), "versions:1.21".to_string()]));
    }

    #[test]
    fn blank_filter_values_are_dropped() {
        let query = SearchQuery {
            loaders: vec!["  ".into(), "fabric".into()],
            categories: vec!["".into()],
            ..Default::default()
        };

        let facets = parse_facets(&query);

        assert_eq!(facets.len(), 2, "остаться должны только тип проекта и fabric");
        assert!(facets.contains(&vec!["categories:fabric".to_string()]));
    }

    #[test]
    fn environment_maps_to_side_facets() {
        let client = SearchQuery {
            environment: Some("client".into()),
            ..Default::default()
        };
        assert!(parse_facets(&client).contains(&vec!["client_side:required".to_string()]));

        let nonsense = SearchQuery {
            environment: Some("что-то".into()),
            ..Default::default()
        };
        assert_eq!(parse_facets(&nonsense).len(), 1);
    }

    #[test]
    fn url_encodes_the_query_and_clamps_the_page() {
        let url = SearchQuery {
            query: "  всё лучшее сразу ".into(),
            limit: 900,
            offset: 40,
            ..Default::default()
        }
        .url();

        assert!(url.starts_with("https://api.modrinth.com/v2/search?"));
        assert!(!url.contains(' '));
        assert!(url.contains(&format!("limit={MAX_LIMIT}")));
        assert!(url.contains("offset=40"));
        assert!(url.contains("index=relevance"));
    }

    #[test]
    fn only_known_sort_orders_reach_the_api() {
        let sorted = |sort: &str| {
            SearchQuery {
                sort: Some(sort.into()),
                ..Default::default()
            }
            .sort()
            .to_string()
        };

        assert_eq!(sorted("downloads"), "downloads");
        assert_eq!(sorted("; drop"), "relevance");
    }

    #[test]
    fn zero_limit_still_asks_for_something() {
        assert_eq!(SearchQuery::default().limit(), 1);
    }

    fn version_json(loaders: &[&str], files: serde_json::Value) -> Version {
        serde_json::from_value(serde_json::json!({
            "id": "abc",
            "project_id": "pack",
            "name": "1.2.3",
            "version_number": "1.2.3",
            "version_type": "release",
            "game_versions": ["1.20.1", "1.20.4"],
            "loaders": loaders,
            "files": files
        }))
        .unwrap()
    }

    #[test]
    fn the_mrpack_wins_over_other_files_of_the_version() {
        let version = version_json(
            &["fabric"],
            serde_json::json!([
                {"url": "https://cdn/server.zip", "filename": "server.zip", "primary": true},
                {"url": "https://cdn/pack.mrpack", "filename": "pack.mrpack", "primary": false}
            ]),
        );

        assert_eq!(version.pack_file().unwrap().filename, "pack.mrpack");
    }

    #[test]
    fn a_version_without_an_mrpack_is_not_installable() {
        let summary = VersionSummary::from(version_json(
            &["fabric"],
            serde_json::json!([{"url": "https://cdn/a.zip", "filename": "a.zip", "primary": true}]),
        ));

        assert!(!summary.supported);
        assert!(summary.file.is_none());
    }

    #[test]
    fn unsupported_loaders_are_not_offered_for_install() {
        let quilt = VersionSummary::from(version_json(
            &["quilt"],
            serde_json::json!([{"url": "https://cdn/p.mrpack", "filename": "p.mrpack", "primary": true}]),
        ));

        assert!(quilt.loader.is_none());
        assert!(!quilt.supported);

        let fabric = VersionSummary::from(version_json(
            &["fabric"],
            serde_json::json!([{"url": "https://cdn/p.mrpack", "filename": "p.mrpack", "primary": true}]),
        ));

        assert_eq!(fabric.loader, Some(LoaderType::Fabric));
        assert_eq!(fabric.minecraft_version.as_deref(), Some("1.20.4"));
        assert!(fabric.supported);
    }

    #[test]
    fn icon_names_keep_the_original_extension() {
        assert_eq!(
            icon_file_name("1KVo5zza", "https://cdn.modrinth.com/data/1KVo5zza/icon.WEBP"),
            "modrinth-1KVo5zza.webp"
        );
        assert_eq!(
            icon_file_name("abc", "https://cdn.modrinth.com/data/abc/icon.png?v=2"),
            "modrinth-abc.png"
        );
        assert_eq!(icon_file_name("abc", "https://cdn.modrinth.com/data/abc/icon"), "modrinth-abc.png");
    }

    #[tokio::test]
    async fn icons_are_only_taken_from_the_modrinth_cdn() {
        assert!(icon("http://cdn.modrinth.com/a.png").await.is_err());
        assert!(icon("https://example.com/a.png").await.is_err());
        assert!(icon("не ссылка").await.is_err());
    }

    #[test]
    fn identifiers_from_the_front_cannot_walk_the_api_path() {
        assert!(segment("../../admin").is_err());
        assert!(segment("pack id").is_err());
        assert!(segment("").is_err());
        assert_eq!(segment(" fabulously-optimized ").unwrap(), "fabulously-optimized");
    }
}
