pub mod manual;

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};
use crate::instance::{LoaderType, PackProvider};
use crate::net::download::DownloadTask;
use crate::net::meta_cache::MetaCache;

pub const SORTS: &[&str] = &["relevance", "downloads", "follows", "newest", "updated"];

pub fn sorts_for(provider: PackProvider) -> Vec<&'static str> {
    match provider {
        PackProvider::Modrinth => SORTS.to_vec(),
        PackProvider::CurseForge => vec!["relevance", "downloads", "updated"],
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub multiple_game_versions: bool,
    pub environment: bool,
    pub blockable_files: bool,
}

pub fn capabilities(provider: PackProvider) -> Capabilities {
    match provider {
        PackProvider::Modrinth => Capabilities {
            multiple_game_versions: true,
            environment: true,
            blockable_files: false,
        },
        PackProvider::CurseForge => Capabilities {
            multiple_game_versions: false,
            environment: false,
            blockable_files: true,
        },
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: PackProvider,
    pub label: &'static str,
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub sorts: Vec<&'static str>,
    pub capabilities: Capabilities,
}

pub fn providers() -> Vec<ProviderInfo> {
    PackProvider::ALL
        .into_iter()
        .map(|provider| {
            let ready = match provider {
                PackProvider::Modrinth => true,
                PackProvider::CurseForge => crate::curseforge::is_available(),
            };

            ProviderInfo {
                id: provider,
                label: provider.label(),
                ready,
                reason: (!ready).then(|| "Лаунчер собран без ключа CurseForge API".to_string()),
                sorts: sorts_for(provider),
                capabilities: capabilities(provider),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchQuery {
    pub provider: PackProvider,
    pub query: String,
    pub categories: Vec<String>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
    pub environment: Option<String>,
    pub sort: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            provider: PackProvider::Modrinth,
            query: String::new(),
            categories: Vec::new(),
            loaders: Vec::new(),
            game_versions: Vec::new(),
            environment: None,
            sort: None,
            offset: 0,
            limit: 20,
        }
    }
}

impl SearchQuery {
    pub fn clean(values: &[String]) -> impl Iterator<Item = &str> {
        values
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
    }

    pub fn sort_key(&self) -> &str {
        let supported = sorts_for(self.provider);

        self.sort
            .as_deref()
            .map(str::trim)
            .filter(|sort| supported.contains(sort))
            .unwrap_or("relevance")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub label: String,
    pub header: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFilters {
    pub categories: Vec<Category>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub url: String,
    pub filename: String,
    pub size: Option<u64>,
    pub hashes: FileHashes,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackHit {
    pub provider: PackProvider,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub author: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub display_categories: Vec<String>,
    pub versions: Vec<String>,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    pub date_modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    pub distribution_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackPage {
    pub hits: Vec<PackHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackVersion {
    pub provider: PackProvider,
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
    pub file: Option<PackFile>,
    pub blocked: bool,
    pub supported: bool,
}

impl PackVersion {
    pub fn unsupported_reason(&self) -> Option<String> {
        if self.supported {
            return None;
        }

        if self.blocked {
            return Some("автор запретил скачивание через сторонние лаунчеры".into());
        }

        if self.loader.is_none() {
            let loaders = match self.loaders.is_empty() {
                true => "не указан".to_string(),
                false => self.loaders.join(", "),
            };

            return Some(format!("неподдерживаемый загрузчик ({loaders})"));
        }

        if self.minecraft_version.is_none() {
            return Some("не указана версия Minecraft".into());
        }

        Some("в версии нет архива пака".into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedFile {
    pub file_name: String,
    pub target_path: String,
    pub website_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
}

impl BlockedFile {
    pub fn found(&self) -> bool {
        self.local_path.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPack {
    pub minecraft_version: String,
    pub loader: LoaderType,
    pub loader_version: Option<String>,
    pub tasks: Vec<DownloadTask>,
    pub paths: Vec<String>,
    pub overrides: Vec<String>,
    pub blocked: Vec<BlockedFile>,
    pub recommended_ram: Option<u32>,
}

pub async fn search(query: &SearchQuery) -> CommandResult<PackPage> {
    match query.provider {
        PackProvider::Modrinth => crate::modrinth::search(query).await,
        PackProvider::CurseForge => crate::curseforge::search(query).await,
    }
}

pub async fn versions(provider: PackProvider, project_id: &str) -> CommandResult<Vec<PackVersion>> {
    match provider {
        PackProvider::Modrinth => crate::modrinth::versions(project_id).await,
        PackProvider::CurseForge => crate::curseforge::versions(project_id).await,
    }
}

pub async fn version(
    provider: PackProvider,
    project_id: &str,
    version_id: &str,
) -> CommandResult<PackVersion> {
    match provider {
        PackProvider::Modrinth => crate::modrinth::version(version_id).await,
        PackProvider::CurseForge => crate::curseforge::version(project_id, version_id).await,
    }
}

pub async fn filters(provider: PackProvider, meta: &MetaCache) -> CommandResult<PackFilters> {
    match provider {
        PackProvider::Modrinth => crate::modrinth::filters(meta).await,
        PackProvider::CurseForge => crate::curseforge::filters(meta).await,
    }
}

pub async fn icon(provider: PackProvider, url: &str) -> CommandResult<Vec<u8>> {
    match provider {
        PackProvider::Modrinth => crate::modrinth::icon(url).await,
        PackProvider::CurseForge => crate::curseforge::icon(url).await,
    }
}

pub fn icon_file_name(provider: PackProvider, project_id: &str, url: &str) -> String {
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

    format!("{}-{project_id}.{extension}", provider.key())
}

pub(crate) async fn fetch_icon(url: &str, hosts: &[&str]) -> CommandResult<Vec<u8>> {
    let parsed = url::Url::parse(url)
        .map_err(|e| CommandError::network("Некорректная ссылка на иконку").with_details(e.to_string()))?;

    let allowed = parsed.scheme() == "https"
        && parsed
            .host_str()
            .is_some_and(|host| hosts.iter().any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}"))));

    if !allowed {
        return Err(CommandError::network(format!(
            "Иконка не из каталога {}: {url}",
            hosts.join(" / ")
        )));
    }

    let response = crate::net::http::client().get(parsed.as_str()).send().await.map_err(|e| {
        CommandError::network("Не удалось скачать иконку").with_details(format!("{url}\n{e}"))
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(crate::net::http::http_status_error(status, url));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curseforge_hides_the_sorts_it_cannot_do() {
        let cf = sorts_for(PackProvider::CurseForge);

        assert!(!cf.contains(&"follows"), "у CurseForge нет подписок");
        assert!(!cf.contains(&"newest"), "у CurseForge нет сортировки по дате создания");
        assert!(cf.contains(&"downloads"));

        assert_eq!(sorts_for(PackProvider::Modrinth).len(), SORTS.len());
    }

    #[test]
    fn an_unsupported_sort_falls_back_to_relevance() {
        let query = |provider, sort: &str| SearchQuery {
            provider,
            sort: Some(sort.into()),
            ..Default::default()
        }
        .sort_key()
        .to_string();

        assert_eq!(query(PackProvider::Modrinth, "follows"), "follows");
        assert_eq!(
            query(PackProvider::CurseForge, "follows"),
            "relevance",
            "сортировку не из списка провайдера подменяем, а не шлём как есть"
        );
        assert_eq!(query(PackProvider::CurseForge, "; drop"), "relevance");
    }

    #[test]
    fn blank_filter_values_are_dropped() {
        let values = vec!["  ".to_string(), "fabric".to_string(), String::new()];

        assert_eq!(SearchQuery::clean(&values).collect::<Vec<_>>(), vec!["fabric"]);
    }

    #[test]
    fn icon_names_carry_the_provider_and_keep_the_extension() {
        assert_eq!(
            icon_file_name(PackProvider::Modrinth, "1KVo5zza", "https://cdn.modrinth.com/data/1KVo5zza/icon.WEBP"),
            "modrinth-1KVo5zza.webp"
        );
        assert_eq!(
            icon_file_name(PackProvider::CurseForge, "925200", "https://media.forgecdn.net/avatars/1182/438/x.png?v=2"),
            "curseforge-925200.png"
        );
        assert_eq!(
            icon_file_name(PackProvider::CurseForge, "925200", "https://media.forgecdn.net/avatars/x"),
            "curseforge-925200.png",
            "без расширения — считаем png"
        );
    }

    #[tokio::test]
    async fn icons_are_only_taken_from_the_allowed_cdn() {
        assert!(fetch_icon("http://media.forgecdn.net/a.png", &["media.forgecdn.net"]).await.is_err());
        assert!(fetch_icon("https://example.com/a.png", &["media.forgecdn.net"]).await.is_err());
        assert!(fetch_icon("не ссылка", &["media.forgecdn.net"]).await.is_err());
    }

    fn version(loader: Option<LoaderType>, file: bool, blocked: bool) -> PackVersion {
        PackVersion {
            provider: PackProvider::CurseForge,
            id: "1".into(),
            project_id: "2".into(),
            name: "1.0".into(),
            version_number: "1.0".into(),
            version_type: "release".into(),
            downloads: 0,
            date_published: None,
            game_versions: vec!["1.20.1".into()],
            loaders: vec!["quilt".into()],
            minecraft_version: Some("1.20.1".into()),
            loader,
            file: file.then(|| PackFile {
                url: "https://edge.forgecdn.net/a.zip".into(),
                filename: "a.zip".into(),
                size: None,
                hashes: FileHashes::default(),
            }),
            blocked,
            supported: loader.is_some() && file,
        }
    }

    #[test]
    fn every_kind_of_unsupported_version_explains_itself() {
        assert!(version(Some(LoaderType::Fabric), true, false).unsupported_reason().is_none());

        let quilt = version(None, true, false).unsupported_reason().unwrap();
        assert!(quilt.contains("quilt"), "в тексте должен быть загрузчик: {quilt}");

        let blocked = version(Some(LoaderType::Fabric), false, true).unsupported_reason().unwrap();
        assert!(blocked.contains("запретил"), "{blocked}");

        let empty = version(Some(LoaderType::Fabric), false, false).unsupported_reason().unwrap();
        assert!(empty.contains("архива"), "{empty}");
    }

    #[test]
    fn providers_travel_with_their_limits() {
        let all = providers();

        assert_eq!(all.len(), 2);

        let modrinth = all.iter().find(|p| p.id == PackProvider::Modrinth).unwrap();
        assert!(modrinth.ready);
        assert!(modrinth.capabilities.multiple_game_versions);
        assert!(!modrinth.capabilities.blockable_files);

        let curseforge = all.iter().find(|p| p.id == PackProvider::CurseForge).unwrap();
        assert!(!curseforge.capabilities.multiple_game_versions);
        assert!(curseforge.capabilities.blockable_files);
    }
}
