pub mod local;
pub mod manual;
pub mod switch;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};
use crate::instance::{LoaderType, PackProvider, PackSource};
use crate::net::download::DownloadTask;
use crate::net::meta_cache::MetaCache;
use crate::text::UiText;

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
    pub reason: Option<UiText>,
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
                reason: (!ready).then(|| UiText::new("catalog.provider.no_api_key")),
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
    /// Mirrors `unsupportedReason()` on the frontend, which shows the same keys in the catalog.
    pub fn unsupported_reason(&self) -> Option<UiText> {
        if self.supported {
            return None;
        }

        if self.blocked {
            return Some(UiText::new("catalog.unsupported.blocked"));
        }

        if self.loader.is_none() {
            let reason = UiText::new("catalog.unsupported.loader");

            return Some(match self.loaders.is_empty() {
                true => {
                    reason.param_text("loaders", UiText::new("catalog.unsupported.loader_unknown"))
                }
                false => reason.param("loaders", self.loaders.join(", ")),
            });
        }

        if self.minecraft_version.is_none() {
            return Some(UiText::new("catalog.unsupported.minecraft"));
        }

        Some(UiText::new("catalog.unsupported.archive"))
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
    pub seed: Vec<crate::castpack::SeedFile>,
    pub delete: Vec<String>,
    pub embedded: Vec<crate::castpack::EmbeddedFile>,
    /// Paths a CastPack sets itself: the overrides of its base pack must not overwrite them.
    pub protected: BTreeSet<String>,
}

/// Where a downloaded modpack archive is kept between installs.
pub fn cached_archive(
    paths: &crate::paths::LauncherPaths,
    pack: &PackSource,
) -> CommandResult<std::path::PathBuf> {
    let key: String = pack
        .version_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(32)
        .collect();

    if key.is_empty() {
        return Err(
            CommandError::manifest("error.reason.modpack.invalid_version")
                .param("version", &pack.version_id),
        );
    }

    let name = format!(
        "{}-{key}.{}",
        pack.provider.key(),
        pack.provider.archive_extension()
    );

    crate::fs_util::child_file(&paths.cache().join("modpacks"), &name)
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
    format!("{}-{project_id}.{}", provider.key(), icon_extension(url))
}

/// The extension of an icon URL, `png` when the URL does not show a sane one.
pub fn icon_extension(url: &str) -> String {
    url.split('?')
        .next()
        .and_then(|path| path.rsplit('/').next())
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .filter(|extension| {
            (1..=5).contains(&extension.len())
                && extension.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or_else(|| "png".to_string())
}

pub(crate) async fn fetch_icon(url: &str, hosts: &[&str]) -> CommandResult<Vec<u8>> {
    let parsed = url::Url::parse(url).map_err(|e| {
        CommandError::network("error.reason.links.invalid_icon")
            .with_details(crate::error::error_chain(&e))
    })?;

    let allowed = parsed.scheme() == "https"
        && parsed.host_str().is_some_and(|host| {
            hosts
                .iter()
                .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
        });

    if !allowed {
        return Err(CommandError::network("error.reason.links.foreign_icon")
            .param("hosts", hosts.join(" / "))
            .param("url", url));
    }

    let response = crate::net::http::client()
        .get(parsed.as_str())
        .send()
        .await
        .map_err(|e| {
            CommandError::network("error.reason.download.icon_failed")
                .with_details(format!("{url}\n{}", crate::error::error_chain(&e)))
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(crate::net::http::http_status_error(status, url));
    }

    if response
        .content_length()
        .is_some_and(|size| size > crate::icons::MAX_SIZE)
    {
        return Err(
            CommandError::download("error.reason.download.icon_too_large").param("url", url),
        );
    }

    let bytes = response.bytes().await.map_err(|e| {
        CommandError::download("error.reason.download.icon_interrupted")
            .param("url", url)
            .with_details(crate::error::error_chain(&e))
    })?;

    if bytes.len() as u64 > crate::icons::MAX_SIZE {
        return Err(
            CommandError::download("error.reason.download.icon_too_large").param("url", url),
        );
    }

    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Param;

    #[test]
    fn curseforge_hides_the_sorts_it_cannot_do() {
        let cf = sorts_for(PackProvider::CurseForge);

        assert!(!cf.contains(&"follows"), "CurseForge has no follows");
        assert!(
            !cf.contains(&"newest"),
            "CurseForge can not sort by creation date"
        );
        assert!(cf.contains(&"downloads"));

        assert_eq!(sorts_for(PackProvider::Modrinth).len(), SORTS.len());
    }

    #[test]
    fn an_unsupported_sort_falls_back_to_relevance() {
        let query = |provider, sort: &str| {
            SearchQuery {
                provider,
                sort: Some(sort.into()),
                ..Default::default()
            }
            .sort_key()
            .to_string()
        };

        assert_eq!(query(PackProvider::Modrinth, "follows"), "follows");
        assert_eq!(
            query(PackProvider::CurseForge, "follows"),
            "relevance",
            "a sort the provider does not list is replaced, not sent as is"
        );
        assert_eq!(query(PackProvider::CurseForge, "; drop"), "relevance");
    }

    #[test]
    fn blank_filter_values_are_dropped() {
        let values = vec!["  ".to_string(), "fabric".to_string(), String::new()];

        assert_eq!(
            SearchQuery::clean(&values).collect::<Vec<_>>(),
            vec!["fabric"]
        );
    }

    #[test]
    fn icon_names_carry_the_provider_and_keep_the_extension() {
        assert_eq!(
            icon_file_name(
                PackProvider::Modrinth,
                "1KVo5zza",
                "https://cdn.modrinth.com/data/1KVo5zza/icon.WEBP"
            ),
            "modrinth-1KVo5zza.webp"
        );
        assert_eq!(
            icon_file_name(
                PackProvider::CurseForge,
                "925200",
                "https://media.forgecdn.net/avatars/1182/438/x.png?v=2"
            ),
            "curseforge-925200.png"
        );
        assert_eq!(
            icon_file_name(
                PackProvider::CurseForge,
                "925200",
                "https://media.forgecdn.net/avatars/x"
            ),
            "curseforge-925200.png",
            "no extension means png"
        );
    }

    #[tokio::test]
    async fn icons_are_only_taken_from_the_allowed_cdn() {
        assert!(
            fetch_icon("http://media.forgecdn.net/a.png", &["media.forgecdn.net"])
                .await
                .is_err()
        );
        assert!(
            fetch_icon("https://example.com/a.png", &["media.forgecdn.net"])
                .await
                .is_err()
        );
        assert!(fetch_icon("не ссылка", &["media.forgecdn.net"])
            .await
            .is_err());
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
        assert!(version(Some(LoaderType::Fabric), true, false)
            .unsupported_reason()
            .is_none());

        let quilt = version(None, true, false).unsupported_reason().unwrap();
        assert_eq!(quilt.key, "catalog.unsupported.loader");
        assert!(
            matches!(&quilt.params["loaders"], Param::Value(loaders) if loaders.contains("quilt")),
            "the text must name the loader: {quilt:?}"
        );

        let blocked = version(Some(LoaderType::Fabric), false, true)
            .unsupported_reason()
            .unwrap();
        assert_eq!(blocked.key, "catalog.unsupported.blocked");

        let empty = version(Some(LoaderType::Fabric), false, false)
            .unsupported_reason()
            .unwrap();
        assert_eq!(empty.key, "catalog.unsupported.archive");
    }

    #[test]
    fn providers_travel_with_their_limits() {
        let all = providers();

        assert_eq!(all.len(), 2);

        let modrinth = all.iter().find(|p| p.id == PackProvider::Modrinth).unwrap();
        assert!(modrinth.ready);
        assert!(modrinth.capabilities.multiple_game_versions);
        assert!(!modrinth.capabilities.blockable_files);

        let curseforge = all
            .iter()
            .find(|p| p.id == PackProvider::CurseForge)
            .unwrap();
        assert!(!curseforge.capabilities.multiple_game_versions);
        assert!(curseforge.capabilities.blockable_files);
    }
}
