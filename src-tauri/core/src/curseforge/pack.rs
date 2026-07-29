use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{relative_key, safe_join};
use crate::instance::LoaderType;
use crate::net::download::DownloadTask;
use crate::packs::{BlockedFile, ResolvedPack};

use super::{post_json, RawFile, RawMod, API};

pub const MANIFEST_ENTRY: &str = "manifest.json";

const MANIFEST_TYPE: &str = "minecraftModpack";

const BATCH: usize = 300;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(default)]
    pub manifest_type: String,
    #[serde(default)]
    pub manifest_version: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub minecraft: Minecraft,
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    #[serde(default = "default_overrides")]
    pub overrides: String,
}

fn default_overrides() -> String {
    "overrides".to_string()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Minecraft {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub mod_loaders: Vec<ModLoader>,
    #[serde(default)]
    pub recommended_ram: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModLoader {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFile {
    #[serde(rename = "projectID")]
    pub project_id: u64,
    #[serde(rename = "fileID")]
    pub file_id: u64,
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> CommandResult<Self> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| {
            CommandError::manifest(format!("Повреждённый {MANIFEST_ENTRY} внутри модпака"))
                .with_details(e.to_string())
        })?;

        manifest.validate()?;

        Ok(manifest)
    }

    fn validate(&self) -> CommandResult<()> {
        if self.manifest_type != MANIFEST_TYPE {
            return Err(CommandError::manifest(format!(
                "Это не модпак Minecraft: {}",
                self.manifest_type
            )));
        }

        if self.manifest_version != 1 {
            return Err(CommandError::manifest(format!(
                "Неизвестная версия формата модпака: {}",
                self.manifest_version
            )));
        }

        Ok(())
    }

    pub fn minecraft_version(&self) -> CommandResult<&str> {
        let version = self.minecraft.version.trim().trim_end_matches('.');

        (!version.is_empty())
            .then_some(version)
            .ok_or_else(|| CommandError::manifest("В модпаке не указана версия Minecraft"))
    }

    pub fn loader(&self) -> CommandResult<(LoaderType, Option<String>)> {
        let minecraft = self.minecraft_version()?;

        let chosen = self
            .minecraft
            .mod_loaders
            .iter()
            .find(|loader| loader.primary)
            .or_else(|| self.minecraft.mod_loaders.first());

        let Some(loader) = chosen else {
            return Ok((LoaderType::Vanilla, None));
        };

        parse_loader(&loader.id, minecraft)
    }
}

fn parse_loader(id: &str, minecraft: &str) -> CommandResult<(LoaderType, Option<String>)> {
    let id = id.trim();

    if id.is_empty() {
        return Ok((LoaderType::Vanilla, None));
    }

    if let Some(version) = id.strip_prefix("neoforge-") {
        let version = version.strip_prefix("1.20.1-").unwrap_or(version);

        return Ok((
            LoaderType::NeoForge,
            Some(crate::meta::neoforge::maven_version(minecraft, version)),
        ));
    }

    if let Some(version) = id.strip_prefix("forge-") {
        return Ok((
            LoaderType::Forge,
            Some(crate::meta::forge::maven_version(minecraft, version)),
        ));
    }

    if let Some(version) = id.strip_prefix("fabric-") {
        return Ok((LoaderType::Fabric, Some(version.to_string())));
    }

    if id.starts_with("quilt-") {
        return Err(CommandError::manifest("Модпаки на Quilt пока не поддерживаются"));
    }

    Err(CommandError::manifest(format!(
        "Модпак собран неизвестным загрузчиком: {id}"
    )))
}

#[derive(Debug, Clone)]
struct ResolvedFile {
    file_id: u64,
    file_name: String,
    target_folder: &'static str,
    website_url: Option<String>,
    url: Option<String>,
    size: Option<u64>,
    sha1: Option<String>,
    required: bool,
}

impl ResolvedFile {
    fn path(&self) -> String {
        match self.required {
            true => format!("{}/{}", self.target_folder, self.file_name),
            false => format!("{}/{}.disabled", self.target_folder, self.file_name),
        }
    }

    fn download_page(&self) -> String {
        match &self.website_url {
            Some(url) => format!("{}/download/{}", url.trim_end_matches('/'), self.file_id),
            None => String::new(),
        }
    }
}

pub async fn resolve(manifest: &Manifest, minecraft_dir: &Path) -> CommandResult<ResolvedPack> {
    let (loader, loader_version) = manifest.loader()?;
    let minecraft_version = manifest.minecraft_version()?.to_string();

    let files = resolve_files(&manifest.files).await?;

    let mut tasks = Vec::new();
    let mut paths = Vec::new();
    let mut blocked = Vec::new();

    for file in files {
        let path = file.path();
        let key = relative_key(&path)?;

        match &file.url {
            Some(url) => {
                tasks.push(DownloadTask::verified(
                    url.clone(),
                    safe_join(minecraft_dir, &key)?,
                    file.size,
                    file.sha1.clone(),
                ));
                paths.push(key);
            }
            None => blocked.push(BlockedFile {
                file_name: file.file_name.clone(),
                target_path: key,
                website_url: file.download_page(),
                sha1: file.sha1.clone(),
                local_path: None,
            }),
        }
    }

    Ok(ResolvedPack {
        minecraft_version,
        loader,
        loader_version,
        tasks,
        paths,
        overrides: vec![manifest.overrides.clone()],
        blocked,
        recommended_ram: manifest.minecraft.recommended_ram,
    })
}

async fn resolve_files(entries: &[ManifestFile]) -> CommandResult<Vec<ResolvedFile>> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let required: BTreeMap<u64, bool> = entries
        .iter()
        .map(|entry| (entry.file_id, entry.required))
        .collect();

    let file_ids: Vec<u64> = required.keys().copied().collect();
    let raw_files = files_by_ids(&file_ids).await?;

    let mod_ids: BTreeSet<u64> = raw_files.iter().map(RawFile::mod_id).collect();
    let projects = mods_by_ids(&mod_ids.into_iter().collect::<Vec<_>>())
        .await
        .unwrap_or_default();

    let mut resolved: Vec<ResolvedFile> = raw_files
        .into_iter()
        .map(|raw| {
            let project = projects.get(&raw.mod_id());

            ResolvedFile {
                file_id: raw.id(),
                file_name: sanitize(raw.file_name()),
                target_folder: project.map(|(folder, _)| *folder).unwrap_or("mods"),
                website_url: project.and_then(|(_, url)| url.clone()),
                url: raw.download_url().map(str::to_string),
                size: raw.size(),
                sha1: raw.sha1(),
                required: required.get(&raw.id()).copied().unwrap_or(true),
            }
        })
        .collect();

    recover_blocked(&mut resolved).await;

    Ok(resolved)
}

async fn recover_blocked(files: &mut [ResolvedFile]) {
    let hashes: Vec<String> = files
        .iter()
        .filter(|file| file.url.is_none())
        .filter_map(|file| file.sha1.clone())
        .collect();

    if hashes.is_empty() {
        return;
    }

    let found = crate::modrinth::files_by_sha1(&hashes).await.unwrap_or_default();

    for file in files.iter_mut().filter(|file| file.url.is_none()) {
        let Some(sha1) = &file.sha1 else { continue };
        let Some(replacement) = found.get(sha1) else { continue };

        file.url = Some(replacement.url.clone());
        file.size = replacement.size.or(file.size);
    }
}

async fn files_by_ids(file_ids: &[u64]) -> CommandResult<Vec<RawFile>> {
    #[derive(Serialize)]
    struct Body<'a> {
        #[serde(rename = "fileIds")]
        file_ids: &'a [u64],
    }

    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        data: Vec<RawFile>,
    }

    let mut collected = Vec::with_capacity(file_ids.len());

    for chunk in file_ids.chunks(BATCH) {
        let response: Response = post_json(&format!("{API}/mods/files"), &Body { file_ids: chunk }).await?;

        collected.extend(response.data);
    }

    Ok(collected)
}

async fn mods_by_ids(mod_ids: &[u64]) -> CommandResult<BTreeMap<u64, (&'static str, Option<String>)>> {
    #[derive(Serialize)]
    struct Body<'a> {
        #[serde(rename = "modIds")]
        mod_ids: &'a [u64],
    }

    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        data: Vec<RawMod>,
    }

    let mut collected = BTreeMap::new();

    for chunk in mod_ids.chunks(BATCH) {
        let response: Response = post_json(&format!("{API}/mods"), &Body { mod_ids: chunk }).await?;

        for project in response.data {
            collected.insert(
                project.id(),
                (project.target_folder(), project.website_url().map(str::to_string)),
            );
        }
    }

    Ok(collected)
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|symbol| match symbol {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            symbol if symbol.is_control() => '_',
            symbol => symbol,
        })
        .collect();

    let cleaned = cleaned.trim().trim_matches('.').trim().to_string();

    match cleaned.is_empty() {
        true => "file".to_string(),
        false => cleaned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: serde_json::Value) -> Manifest {
        let mut json = json;
        let object = json.as_object_mut().unwrap();

        object.entry("manifestType").or_insert(serde_json::json!(MANIFEST_TYPE));
        object.entry("manifestVersion").or_insert(serde_json::json!(1));

        Manifest::parse(&serde_json::to_vec(&json).unwrap()).unwrap()
    }

    fn parse(json: serde_json::Value) -> CommandResult<Manifest> {
        Manifest::parse(&serde_json::to_vec(&json).unwrap())
    }

    fn with_loader(id: &str, minecraft: &str) -> CommandResult<(LoaderType, Option<String>)> {
        manifest(serde_json::json!({
            "minecraft": {"version": minecraft, "modLoaders": [{"id": id, "primary": true}]}
        }))
        .loader()
    }

    fn real_manifest() -> Manifest {
        manifest(serde_json::json!({
            "minecraft": {
                "version": "26.1.2",
                "modLoaders": [{"id": "fabric-0.19.3", "primary": true}]
            },
            "name": "Fabulously Optimized",
            "version": "13.3.0",
            "author": "robotkoer",
            "files": [
                {"projectID": 1285307, "fileID": 8385054, "required": true, "isLocked": false},
                {"projectID": 556408, "fileID": 7854083, "required": true, "isLocked": false}
            ],
            "overrides": "overrides"
        }))
    }

    #[test]
    fn a_real_manifest_is_read_field_by_field() {
        let manifest = real_manifest();

        assert_eq!(manifest.name, "Fabulously Optimized");
        assert_eq!(manifest.version, "13.3.0");
        assert_eq!(manifest.minecraft_version().unwrap(), "26.1.2");
        assert_eq!(manifest.loader().unwrap(), (LoaderType::Fabric, Some("0.19.3".into())));
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.overrides, "overrides");
    }

    #[test]
    fn a_manifest_of_another_type_or_format_is_rejected() {
        assert!(parse(serde_json::json!({"manifestType": "minecraftInstance", "manifestVersion": 1})).is_err());
        assert!(parse(serde_json::json!({"manifestType": MANIFEST_TYPE, "manifestVersion": 2})).is_err());
        assert!(parse(serde_json::json!({"manifestVersion": 1})).is_err(), "тип обязателен");
        assert!(parse(serde_json::json!({"manifestType": MANIFEST_TYPE, "manifestVersion": 1})).is_ok());
    }

    #[test]
    fn broken_json_is_reported_as_a_manifest_problem() {
        assert_eq!(Manifest::parse(b"{ not json").unwrap_err().code, "MANIFEST_INVALID");
    }

    #[test]
    fn forge_versions_gain_the_minecraft_prefix() {
        assert_eq!(
            with_loader("forge-47.2.0", "1.20.1").unwrap(),
            (LoaderType::Forge, Some("1.20.1-47.2.0".into()))
        );
        assert_eq!(
            with_loader("forge-1.20.1-47.2.0", "1.20.1").unwrap(),
            (LoaderType::Forge, Some("1.20.1-47.2.0".into())),
            "уже полную версию не удваиваем"
        );
    }

    #[test]
    fn neoforge_is_untangled_from_the_curseforge_naming_mess() {
        assert_eq!(
            with_loader("neoforge-21.1.66", "1.21.1").unwrap(),
            (LoaderType::NeoForge, Some("21.1.66".into()))
        );

        assert_eq!(
            with_loader("neoforge-1.20.1-47.1.106", "1.20.1").unwrap(),
            (LoaderType::NeoForge, Some("1.20.1-47.1.106".into()))
        );
        assert_eq!(
            with_loader("neoforge-47.1.106", "1.20.1").unwrap(),
            (LoaderType::NeoForge, Some("1.20.1-47.1.106".into()))
        );
    }

    #[test]
    fn the_primary_loader_wins_over_the_rest() {
        let pack = manifest(serde_json::json!({
            "minecraft": {
                "version": "1.20.1",
                "modLoaders": [
                    {"id": "fabric-0.15.7", "primary": false},
                    {"id": "forge-47.2.0", "primary": true}
                ]
            }
        }));

        assert_eq!(pack.loader().unwrap(), (LoaderType::Forge, Some("1.20.1-47.2.0".into())));
    }

    #[test]
    fn without_a_primary_flag_the_first_loader_is_taken() {
        let pack = manifest(serde_json::json!({
            "minecraft": {"version": "1.20.1", "modLoaders": [{"id": "fabric-0.15.7"}]}
        }));

        assert_eq!(pack.loader().unwrap(), (LoaderType::Fabric, Some("0.15.7".into())));
    }

    #[test]
    fn a_pack_without_loaders_is_plain_vanilla() {
        let pack = manifest(serde_json::json!({"minecraft": {"version": "1.20.1"}}));

        assert_eq!(pack.loader().unwrap(), (LoaderType::Vanilla, None));
    }

    #[test]
    fn loaders_we_cannot_install_are_reported_by_name() {
        let quilt = with_loader("quilt-0.23.1", "1.20.1").unwrap_err();
        assert!(quilt.message.contains("Quilt"));

        let unknown = with_loader("babric-1.0", "1.20.1").unwrap_err();
        assert!(unknown.message.contains("babric-1.0"), "в тексте должен быть сам id");
    }

    #[test]
    fn a_mysterious_trailing_dot_is_trimmed_from_the_game_version() {
        let pack = manifest(serde_json::json!({"minecraft": {"version": "1.12.2."}}));

        assert_eq!(pack.minecraft_version().unwrap(), "1.12.2");
    }

    #[test]
    fn a_pack_without_a_minecraft_version_is_rejected() {
        let pack = manifest(serde_json::json!({
            "minecraft": {"modLoaders": [{"id": "forge-47.2.0"}]}
        }));

        assert!(pack.minecraft_version().is_err());
        assert!(pack.loader().is_err());
    }

    #[test]
    fn overrides_default_to_the_usual_folder() {
        assert_eq!(manifest(serde_json::json!({})).overrides, "overrides");
        assert_eq!(
            manifest(serde_json::json!({"overrides": "client-overrides"})).overrides,
            "client-overrides"
        );
    }

    #[test]
    fn files_are_required_unless_the_manifest_says_otherwise() {
        let pack = manifest(serde_json::json!({
            "files": [
                {"projectID": 1, "fileID": 2},
                {"projectID": 3, "fileID": 4, "required": false}
            ]
        }));

        assert!(pack.files[0].required, "молчание — значит обязательный");
        assert!(!pack.files[1].required);
    }

    fn resolved(file_name: &str, required: bool) -> ResolvedFile {
        ResolvedFile {
            file_id: 8287120,
            file_name: sanitize(file_name),
            target_folder: "mods",
            website_url: None,
            url: Some("https://edge.forgecdn.net/files/1/1/a.jar".into()),
            size: None,
            sha1: None,
            required,
        }
    }

    #[test]
    fn optional_files_are_installed_switched_off() {
        assert_eq!(resolved("jei.jar", true).path(), "mods/jei.jar");
        assert_eq!(resolved("extra.jar", false).path(), "mods/extra.jar.disabled");
    }

    #[test]
    fn file_names_from_the_api_cannot_walk_out_of_the_instance() {
        assert_eq!(sanitize("../../evil.jar"), "_.._evil.jar");
        assert_eq!(sanitize("C:\\windows\\system32.dll"), "C__windows_system32.dll");
        assert_eq!(sanitize("нормальный-мод_1.2.jar"), "нормальный-мод_1.2.jar");
        assert_eq!(sanitize("  ..  "), "file");
        assert_eq!(sanitize(""), "file");

        assert!(safe_join(Path::new("/mc"), "mods/../../escape.jar").is_err());
    }

    #[test]
    fn the_manual_download_link_points_at_the_exact_file() {
        let file = ResolvedFile {
            website_url: Some("https://www.curseforge.com/minecraft/mc-mods/entityculling/".into()),
            ..resolved("entityculling.jar", true)
        };

        assert_eq!(
            file.download_page(),
            "https://www.curseforge.com/minecraft/mc-mods/entityculling/download/8287120"
        );

        assert!(resolved("x.jar", true).download_page().is_empty(), "без страницы проекта ссылки нет");
    }

    #[test]
    fn a_world_lands_in_saves() {
        let world = ResolvedFile {
            target_folder: "saves",
            ..resolved("map.zip", true)
        };

        assert_eq!(world.path(), "saves/map.zip");
    }

    #[tokio::test]
    async fn a_pack_without_files_resolves_to_nothing_to_download() {
        let pack = real_manifest();
        let empty = Manifest {
            files: Vec::new(),
            ..pack
        };

        let resolved = resolve(&empty, Path::new("/mc")).await.unwrap();

        assert!(resolved.tasks.is_empty());
        assert!(resolved.paths.is_empty());
        assert!(resolved.blocked.is_empty());
        assert_eq!(resolved.loader, LoaderType::Fabric);
        assert_eq!(resolved.minecraft_version, "26.1.2");
        assert_eq!(resolved.overrides, vec!["overrides"]);
    }

    #[tokio::test]
    #[ignore = "ходит в сеть"]
    async fn a_real_pack_resolves_into_downloads_and_a_manual_list() {
        let manifest = Manifest {
            files: vec![
                ManifestFile {
                    project_id: 448233,
                    file_id: 8287120,
                    required: true,
                },
                ManifestFile {
                    project_id: 1285307,
                    file_id: 8385054,
                    required: true,
                },
            ],
            ..real_manifest()
        };

        let resolved = resolve(&manifest, Path::new("/mc")).await.unwrap();

        assert_eq!(
            resolved.tasks.len() + resolved.blocked.len(),
            2,
            "каждый файл должен либо качаться, либо попасть в список ручных"
        );
        assert_eq!(resolved.tasks.len(), resolved.paths.len());

        assert!(
            resolved.blocked.is_empty(),
            "замену на Modrinth не нашли: {:?}",
            resolved.blocked
        );
        assert!(resolved
            .tasks
            .iter()
            .any(|task| task.url.contains("modrinth.com")));
    }

    #[test]
    fn the_recommended_memory_is_carried_over_when_the_pack_states_it() {
        let pack = manifest(serde_json::json!({
            "minecraft": {"version": "1.20.1", "recommendedRam": 8192}
        }));

        assert_eq!(pack.minecraft.recommended_ram, Some(8192));
        assert_eq!(real_manifest().minecraft.recommended_ram, None);
    }
}
