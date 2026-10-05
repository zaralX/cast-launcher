use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{case_collision, relative_key, safe_join};
use crate::instance::{LoaderType, PackProvider};
use crate::net::download::DownloadTask;
use crate::text::UiText;

use super::{https_url, SCHEMA_VERSION};

pub const MAX_ENTRIES: usize = 512;

pub const MAX_FILE_ENTRIES: usize = 4096;

/// Where a manifest came from: the catalog serves it by link, a `.cast` file carries it inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Catalog,
    File,
}

impl Origin {
    fn max_entries(self) -> usize {
        match self {
            Self::Catalog => MAX_ENTRIES,
            Self::File => MAX_FILE_ENTRIES,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileMode {
    #[default]
    Always,
    Once,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderSpec {
    #[serde(rename = "type")]
    pub loader: LoaderType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseSpec {
    pub provider: PackProvider,
    pub project_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<PackProvider>,
    pub project_id: String,
    pub version_id: String,
    pub url: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModRef<'a> {
    Catalog {
        provider: PackProvider,
        project_id: &'a str,
        version_id: &'a str,
        optional: bool,
    },
    Direct {
        url: &'a str,
        key: String,
        sha1: &'a str,
        size: Option<u64>,
    },
}

impl ModEntry {
    pub fn reference(&self) -> CommandResult<ModRef<'_>> {
        let url = self.url.trim();

        match self.provider {
            Some(provider) if url.is_empty() => {
                let project_id = self.project_id.trim();
                let version_id = self.version_id.trim();

                if project_id.is_empty() || version_id.is_empty() {
                    return Err(CommandError::manifest("error.reason.castpack.mod_no_ids")
                        .param("provider", provider.label()));
                }

                Ok(ModRef::Catalog {
                    provider,
                    project_id,
                    version_id,
                    optional: self.optional,
                })
            }
            Some(_) => Err(
                CommandError::manifest("error.reason.castpack.mod_both_sources").param("url", url),
            ),
            None if url.is_empty() => Err(CommandError::manifest(
                "error.reason.castpack.mod_no_source",
            )),
            None => {
                https_url(url)?;

                let sha1 = self
                    .sha1
                    .as_deref()
                    .map(str::trim)
                    .filter(|hash| !hash.is_empty())
                    .ok_or_else(|| {
                        CommandError::manifest("error.reason.castpack.mod_no_sha1")
                            .param("url", url)
                    })?;

                let key = relative_key(&self.path)?;
                let key = match self.optional {
                    true => format!("{key}.disabled"),
                    false => key,
                };

                Ok(ModRef::Direct {
                    url,
                    key,
                    sha1,
                    size: self.size,
                })
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub embedded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    pub mode: FileMode,
}

/// A file a `.cast` carries inside instead of a link to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFile {
    pub key: String,
    pub sha1: String,
    pub size: Option<u64>,
    pub mode: FileMode,
}

impl FileEntry {
    pub fn key(&self) -> CommandResult<String> {
        relative_key(&self.path)
    }

    fn checked(&self, origin: Origin) -> CommandResult<(String, Option<&str>, &str)> {
        let key = self.key()?;

        let sha1 = self
            .sha1
            .as_deref()
            .map(str::trim)
            .filter(|hash| !hash.is_empty())
            .ok_or_else(|| {
                CommandError::manifest("error.reason.castpack.file_no_sha1")
                    .param("path", &self.path)
            })?;

        if !self.embedded {
            return Ok((key, Some(https_url(&self.url)?), sha1));
        }

        if origin == Origin::Catalog {
            return Err(
                CommandError::manifest("error.reason.castpack.embedded_in_catalog")
                    .param("path", &self.path),
            );
        }

        if !self.url.trim().is_empty() {
            return Err(
                CommandError::manifest("error.reason.castpack.file_both_sources")
                    .param("path", &self.path),
            );
        }

        Ok((key, None, sha1))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PackSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_ram: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub changelog: String,
    pub minecraft: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader: Option<LoaderSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<BaseSpec>,
    pub mods: Vec<ModEntry>,
    pub files: Vec<FileEntry>,
    pub delete: Vec<String>,
    pub settings: PackSettings,
}

#[derive(Debug, Clone)]
pub struct SeedFile {
    pub key: String,
    pub task: DownloadTask,
}

/// An unknown enum value means a newer format, not a broken file.
pub fn json_error(error: serde_json::Error, corrupted: &str) -> CommandError {
    let message = error.to_string();

    match unknown_variant(&message) {
        Some(value) => CommandError::unsupported("error.reason.castpack.unknown_value")
            .param("value", value)
            .with_details(message),
        None => CommandError::manifest(corrupted).with_details(message),
    }
}

fn unknown_variant(message: &str) -> Option<&str> {
    message.strip_prefix("unknown variant `")?.split('`').next()
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> CommandResult<Self> {
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|e| json_error(e, "error.reason.castpack.manifest_corrupted"))?;

        manifest.validate(Origin::Catalog)?;

        Ok(manifest)
    }

    pub fn validate(&self, origin: Origin) -> CommandResult<()> {
        if self.schema_version == 0 || self.schema_version > SCHEMA_VERSION {
            return Err(
                CommandError::unsupported("error.reason.castpack.manifest_version")
                    .param("version", self.schema_version)
                    .param("supported", SCHEMA_VERSION),
            );
        }

        for (field, value) in [
            ("id", &self.id),
            ("name", &self.name),
            ("version", &self.version),
        ] {
            if value.trim().is_empty() {
                return Err(CommandError::manifest("error.reason.castpack.empty_field")
                    .param("field", field));
            }
        }

        if self.base.is_none() && self.minecraft.trim().is_empty() {
            return Err(CommandError::manifest(
                "error.reason.castpack.minecraft_required",
            ));
        }

        let limit = origin.max_entries();

        for (what, count) in [
            ("error.reason.castpack.entries.mods", self.mods.len()),
            ("error.reason.castpack.entries.files", self.files.len()),
            ("error.reason.castpack.entries.delete", self.delete.len()),
        ] {
            if count > limit {
                return Err(CommandError::manifest("error.reason.castpack.too_many")
                    .param_text("what", UiText::new(what))
                    .param("count", count)
                    .param("limit", limit));
            }
        }

        if let Some(base) = &self.base {
            if base.project_id.trim().is_empty() || base.version_id.trim().is_empty() {
                return Err(CommandError::manifest("error.reason.castpack.base_no_ids"));
            }
        }

        let mut keys = Vec::new();

        for entry in &self.mods {
            if let ModRef::Direct { key, .. } = entry.reference()? {
                keys.push(key);
            }
        }

        for entry in &self.files {
            keys.push(entry.checked(origin)?.0);
        }

        if let Some((first, second)) = case_collision(keys.iter().map(String::as_str)) {
            return Err(
                CommandError::manifest("error.reason.castpack.case_collision")
                    .param("first", first)
                    .param("second", second),
            );
        }

        self.delete_keys()?;

        Ok(())
    }

    pub fn loader(&self) -> Option<(LoaderType, Option<String>)> {
        self.loader.as_ref().map(|spec| {
            let version = spec
                .version
                .as_deref()
                .map(str::trim)
                .filter(|version| !version.is_empty())
                .map(str::to_string);

            (spec.loader, version)
        })
    }

    pub fn minecraft_version(&self) -> Option<&str> {
        Some(self.minecraft.trim()).filter(|version| !version.is_empty())
    }

    pub fn catalog_mods(&self) -> CommandResult<Vec<ModRef<'_>>> {
        self.mods
            .iter()
            .map(ModEntry::reference)
            .filter(|entry| matches!(entry, Ok(ModRef::Catalog { .. }) | Err(_)))
            .collect()
    }

    pub fn direct_mods(&self, minecraft_dir: &Path) -> CommandResult<Vec<(String, DownloadTask)>> {
        let mut files = Vec::new();

        for entry in &self.mods {
            let ModRef::Direct {
                url,
                key,
                sha1,
                size,
            } = entry.reference()?
            else {
                continue;
            };

            let task = DownloadTask::verified(
                url.to_string(),
                safe_join(minecraft_dir, &key)?,
                size,
                Some(sha1.to_string()),
            );

            files.push((key, task));
        }

        Ok(files)
    }

    pub fn owned_files(&self, minecraft_dir: &Path) -> CommandResult<Vec<(String, DownloadTask)>> {
        self.files_of(FileMode::Always, minecraft_dir)
    }

    pub fn seed_files(&self, minecraft_dir: &Path) -> CommandResult<Vec<SeedFile>> {
        Ok(self
            .files_of(FileMode::Once, minecraft_dir)?
            .into_iter()
            .map(|(key, task)| SeedFile { key, task })
            .collect())
    }

    fn files_of(
        &self,
        mode: FileMode,
        minecraft_dir: &Path,
    ) -> CommandResult<Vec<(String, DownloadTask)>> {
        let mut files = Vec::new();

        for entry in self.files.iter().filter(|entry| entry.mode == mode) {
            let (key, Some(url), sha1) = entry.checked(Origin::File)? else {
                continue;
            };

            let task = DownloadTask::verified(
                url.to_string(),
                safe_join(minecraft_dir, &key)?,
                entry.size,
                Some(sha1.to_string()),
            );

            files.push((key, task));
        }

        Ok(files)
    }

    pub fn embedded_files(&self) -> CommandResult<Vec<EmbeddedFile>> {
        let mut files = Vec::new();

        for entry in &self.files {
            let (key, None, sha1) = entry.checked(Origin::File)? else {
                continue;
            };

            files.push(EmbeddedFile {
                key,
                sha1: sha1.to_ascii_lowercase(),
                size: entry.size,
                mode: entry.mode,
            });
        }

        Ok(files)
    }

    pub fn delete_keys(&self) -> CommandResult<BTreeSet<String>> {
        self.delete.iter().map(|path| relative_key(path)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest(value: serde_json::Value) -> Manifest {
        let mut value = value;
        let object = value.as_object_mut().unwrap();

        object
            .entry("schemaVersion")
            .or_insert(json!(SCHEMA_VERSION));
        object.entry("id").or_insert(json!("zaralx-rpg"));
        object.entry("name").or_insert(json!("zaralX RPG"));
        object.entry("version").or_insert(json!("1.0.0"));

        Manifest::parse(&serde_json::to_vec(&value).unwrap()).unwrap()
    }

    fn parse(value: serde_json::Value) -> CommandResult<Manifest> {
        let mut value = value;
        let object = value.as_object_mut().unwrap();

        object
            .entry("schemaVersion")
            .or_insert(json!(SCHEMA_VERSION));
        object.entry("id").or_insert(json!("zaralx-rpg"));
        object.entry("name").or_insert(json!("zaralX RPG"));
        object.entry("version").or_insert(json!("1.0.0"));

        Manifest::parse(&serde_json::to_vec(&value).unwrap())
    }

    fn mc() -> &'static Path {
        Path::new("/mc")
    }

    #[test]
    fn a_manifest_of_another_schema_is_rejected() {
        let other = json!({"schemaVersion": 99, "minecraft": "1.20.1"});
        let error = Manifest::parse(&serde_json::to_vec(&other).unwrap()).unwrap_err();

        assert_eq!(error.code, crate::error::ErrorCode::Unsupported);
        assert!(error.text.mentions("99"));
    }

    #[test]
    fn broken_json_is_reported_as_a_manifest_problem() {
        assert_eq!(
            Manifest::parse(b"{ not json").unwrap_err().code,
            crate::error::ErrorCode::ManifestInvalid
        );
    }

    #[test]
    fn without_a_base_pack_the_minecraft_version_is_required() {
        assert!(parse(json!({"mods": []})).is_err());
        assert!(parse(json!({"minecraft": "1.20.1"})).is_ok());

        let with_base = parse(json!({
            "base": {"provider": "modrinth", "projectId": "1KVo5zza", "versionId": "abc"}
        }));
        assert!(with_base.is_ok(), "the modpack provides the game version");
    }

    #[test]
    fn a_base_pack_without_ids_is_rejected() {
        assert!(parse(json!({
            "base": {"provider": "modrinth", "projectId": "", "versionId": "abc"}
        }))
        .is_err());
    }

    #[test]
    fn empty_meta_fields_are_rejected_by_name() {
        let value = json!({"schemaVersion": SCHEMA_VERSION, "id": "a", "name": "  ", "version": "1", "minecraft": "1.20.1"});
        let error = Manifest::parse(&serde_json::to_vec(&value).unwrap()).unwrap_err();

        assert!(error.text.mentions("name"), "{}", error);
    }

    #[test]
    fn a_catalog_mod_needs_both_ids() {
        assert!(parse(json!({
            "minecraft": "1.20.1",
            "mods": [{"provider": "modrinth", "projectId": "AANobbMI", "versionId": "xyz"}]
        }))
        .is_ok());

        assert!(parse(json!({
            "minecraft": "1.20.1",
            "mods": [{"provider": "modrinth", "projectId": "AANobbMI"}]
        }))
        .is_err());
    }

    #[test]
    fn a_direct_mod_without_a_hash_is_rejected() {
        let no_hash = parse(json!({
            "minecraft": "1.20.1",
            "mods": [{"url": "https://cdn.zaralx.ru/core.jar", "path": "mods/core.jar"}]
        }));

        assert!(no_hash.unwrap_err().text.mentions("sha1"));

        assert!(parse(json!({
            "minecraft": "1.20.1",
            "mods": [{"url": "https://cdn.zaralx.ru/core.jar", "path": "mods/core.jar", "sha1": "aaa"}]
        }))
        .is_ok());
    }

    #[test]
    fn plain_http_never_reaches_the_download_queue() {
        assert!(parse(json!({
            "minecraft": "1.20.1",
            "mods": [{"url": "http://cdn.zaralx.ru/core.jar", "path": "mods/core.jar", "sha1": "aaa"}]
        }))
        .is_err());

        assert!(parse(json!({
            "minecraft": "1.20.1",
            "files": [{"path": "options.txt", "url": "ftp://x/options.txt", "sha1": "aaa"}]
        }))
        .is_err());
    }

    #[test]
    fn a_mod_cannot_be_both_a_catalog_entry_and_a_link() {
        assert!(parse(json!({
            "minecraft": "1.20.1",
            "mods": [{
                "provider": "modrinth", "projectId": "AANobbMI", "versionId": "xyz",
                "url": "https://cdn.zaralx.ru/core.jar", "path": "mods/core.jar", "sha1": "aaa"
            }]
        }))
        .is_err());
    }

    #[test]
    fn nothing_from_a_manifest_can_escape_the_game_directory() {
        for evil in [
            json!({"minecraft": "1.20.1", "mods": [{"url": "https://x/a.jar", "path": "../../a.jar", "sha1": "aaa"}]}),
            json!({"minecraft": "1.20.1", "files": [{"path": "../instance.json", "url": "https://x/a", "sha1": "aaa"}]}),
            json!({"minecraft": "1.20.1", "delete": ["../../config.json"]}),
        ] {
            assert!(parse(evil).is_err());
        }
    }

    #[test]
    fn optional_mods_land_switched_off() {
        let pack = manifest(json!({
            "minecraft": "1.20.1",
            "mods": [{
                "url": "https://cdn.zaralx.ru/shaders.jar", "path": "mods/shaders.jar",
                "sha1": "aaa", "optional": true
            }]
        }));

        let direct = pack.direct_mods(mc()).unwrap();

        assert_eq!(direct[0].0, "mods/shaders.jar.disabled");
        assert!(direct[0].1.destination.ends_with("shaders.jar.disabled"));
    }

    #[test]
    fn files_are_split_by_mode() {
        let pack = manifest(json!({
            "minecraft": "1.20.1",
            "files": [
                {"path": "options.txt", "url": "https://x/options.txt", "sha1": "aaa", "mode": "once"},
                {"path": "config/rpg.toml", "url": "https://x/rpg.toml", "sha1": "bbb"},
                {"path": "servers.dat", "url": "https://x/servers.dat", "sha1": "ccc", "mode": "once"}
            ]
        }));

        let owned = pack.owned_files(mc()).unwrap();
        let seeded = pack.seed_files(mc()).unwrap();

        assert_eq!(
            owned.len(),
            1,
            "without a mode the file belongs to the pack"
        );
        assert_eq!(owned[0].0, "config/rpg.toml");

        let keys: Vec<_> = seeded.iter().map(|file| file.key.clone()).collect();
        assert_eq!(keys, vec!["options.txt", "servers.dat"]);
    }

    #[test]
    fn a_direct_mod_carries_its_hash_and_size_into_the_download() {
        let pack = manifest(json!({
            "minecraft": "1.20.1",
            "mods": [{
                "url": "https://cdn.zaralx.ru/core-1.2.jar", "path": "mods/core-1.2.jar",
                "sha1": "abc", "size": 4096
            }]
        }));

        let (key, task) = pack.direct_mods(mc()).unwrap().remove(0);

        assert_eq!(key, "mods/core-1.2.jar");
        assert_eq!(task.sha1.as_deref(), Some("abc"));
        assert_eq!(task.size, Some(4096));
        assert_eq!(task.destination, mc().join("mods").join("core-1.2.jar"));
    }

    #[test]
    fn catalog_mods_are_listed_apart_from_direct_ones() {
        let pack = manifest(json!({
            "minecraft": "1.20.1",
            "mods": [
                {"provider": "curseforge", "projectId": "238222", "versionId": "5432101"},
                {"url": "https://cdn.zaralx.ru/core.jar", "path": "mods/core.jar", "sha1": "aaa"}
            ]
        }));

        assert_eq!(pack.catalog_mods().unwrap().len(), 1);
        assert_eq!(pack.direct_mods(mc()).unwrap().len(), 1);
    }

    #[test]
    fn delete_paths_are_normalised_and_deduplicated() {
        let pack = manifest(json!({
            "minecraft": "1.20.1",
            "delete": ["mods\\old.jar", "mods/old.jar", "config/x.toml"]
        }));

        let keys = pack.delete_keys().unwrap();

        assert_eq!(keys.len(), 2);
        assert!(keys.contains("mods/old.jar"));
    }

    #[test]
    fn the_loader_is_optional_and_trimmed() {
        assert_eq!(manifest(json!({"minecraft": "1.20.1"})).loader(), None);

        let forge = manifest(json!({
            "minecraft": "1.20.1",
            "loader": {"type": "forge", "version": "  1.20.1-47.2.0  "}
        }));
        assert_eq!(
            forge.loader(),
            Some((LoaderType::Forge, Some("1.20.1-47.2.0".into())))
        );

        let bare = manifest(json!({"minecraft": "1.20.1", "loader": {"type": "fabric"}}));
        assert_eq!(bare.loader(), Some((LoaderType::Fabric, None)));
    }

    #[test]
    fn too_many_entries_are_refused_before_anything_is_downloaded() {
        let many: Vec<_> = (0..MAX_ENTRIES + 1)
            .map(|i| json!({"url": format!("https://x/{i}.jar"), "path": format!("mods/{i}.jar"), "sha1": "a"}))
            .collect();

        let error = parse(json!({"minecraft": "1.20.1", "mods": many})).unwrap_err();
        assert!(error.text.mentions("too_many"), "{error}");
    }

    fn from_file(value: serde_json::Value) -> CommandResult<Manifest> {
        let mut value = value;
        let object = value.as_object_mut().unwrap();

        object
            .entry("schemaVersion")
            .or_insert(json!(SCHEMA_VERSION));
        object.entry("id").or_insert(json!("zaralx-rpg"));
        object.entry("name").or_insert(json!("zaralX RPG"));
        object.entry("version").or_insert(json!("1.0.0"));
        object.entry("minecraft").or_insert(json!("1.20.1"));

        let manifest: Manifest = serde_json::from_value(value).unwrap();
        manifest.validate(Origin::File)?;
        Ok(manifest)
    }

    #[test]
    fn a_missing_schema_version_is_not_taken_for_the_current_one() {
        let bare = json!({"id": "a", "name": "a", "version": "1", "minecraft": "1.20.1"});
        let error = Manifest::parse(&serde_json::to_vec(&bare).unwrap()).unwrap_err();

        assert_eq!(error.code, crate::error::ErrorCode::Unsupported);
    }

    #[test]
    fn an_unknown_value_asks_for_a_newer_launcher() {
        let quilt = parse(json!({"minecraft": "1.20.1", "loader": {"type": "quilt"}})).unwrap_err();

        assert_eq!(quilt.code, crate::error::ErrorCode::Unsupported);
        assert!(quilt.text.mentions("unknown_value"), "{quilt}");
        assert!(quilt.text.mentions("quilt"), "{quilt}");

        let provider = parse(json!({
            "minecraft": "1.20.1",
            "mods": [{"provider": "github", "projectId": "a", "versionId": "b"}]
        }))
        .unwrap_err();
        assert_eq!(provider.code, crate::error::ErrorCode::Unsupported);
    }

    #[test]
    fn embedded_files_only_come_inside_a_cast_file() {
        let embedded = json!({
            "minecraft": "1.20.1",
            "files": [{"path": "mods/private.jar", "embedded": true, "sha1": "AAA"}]
        });

        let error = parse(embedded.clone()).unwrap_err();
        assert!(error.text.mentions("embedded_in_catalog"), "{error}");

        let pack = from_file(embedded).unwrap();
        let files = pack.embedded_files().unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].key, "mods/private.jar");
        assert_eq!(files[0].sha1, "aaa", "hashes compare in lowercase");
        assert!(pack.owned_files(mc()).unwrap().is_empty());
    }

    #[test]
    fn an_embedded_file_needs_a_hash_and_no_link() {
        assert!(from_file(json!({
            "files": [{"path": "mods/private.jar", "embedded": true}]
        }))
        .is_err());

        let both = from_file(json!({
            "files": [{"path": "a.txt", "embedded": true, "url": "https://x/a.txt", "sha1": "a"}]
        }))
        .unwrap_err();
        assert!(both.text.mentions("file_both_sources"), "{both}");
    }

    #[test]
    fn linked_and_embedded_files_are_split() {
        let pack = from_file(json!({
            "files": [
                {"path": "config/a.toml", "url": "https://x/a.toml", "sha1": "a"},
                {"path": "config/b.toml", "embedded": true, "sha1": "b"},
                {"path": "options.txt", "embedded": true, "sha1": "c", "mode": "once"}
            ]
        }))
        .unwrap();

        assert_eq!(pack.owned_files(mc()).unwrap().len(), 1);
        assert!(pack.seed_files(mc()).unwrap().is_empty());

        let embedded = pack.embedded_files().unwrap();
        assert_eq!(embedded.len(), 2);
        assert_eq!(embedded[1].mode, FileMode::Once);
    }

    #[test]
    fn paths_differing_only_in_case_are_one_file_on_windows() {
        let error = from_file(json!({
            "files": [
                {"path": "config/A.toml", "embedded": true, "sha1": "a"},
                {"path": "config/a.toml", "embedded": true, "sha1": "b"}
            ]
        }))
        .unwrap_err();

        assert!(error.text.mentions("case_collision"), "{error}");
    }

    #[test]
    fn a_cast_file_may_list_more_than_a_catalog_manifest() {
        let many: Vec<_> = (0..MAX_ENTRIES + 1)
            .map(|i| json!({"path": format!("config/{i}.toml"), "embedded": true, "sha1": "a"}))
            .collect();

        assert!(from_file(json!({"files": many})).is_ok());
    }

    #[test]
    fn an_embedded_file_keeps_no_empty_link_in_json() {
        let pack = from_file(json!({
            "files": [{"path": "config/b.toml", "embedded": true, "sha1": "b"}]
        }))
        .unwrap();

        let written = serde_json::to_value(&pack).unwrap();

        assert_eq!(written["files"][0]["embedded"], true);
        assert!(written["files"][0].get("url").is_none());
    }

    #[test]
    fn a_manifest_survives_a_json_round_trip() {
        let pack = manifest(json!({
            "minecraft": "1.20.1",
            "loader": {"type": "neoforge", "version": "21.1.243"},
            "base": {"provider": "curseforge", "projectId": "925200", "versionId": "5432100"},
            "mods": [{"provider": "modrinth", "projectId": "AANobbMI", "versionId": "xyz"}],
            "files": [{"path": "options.txt", "url": "https://x/o.txt", "sha1": "aaa", "mode": "once"}],
            "delete": ["mods/old.jar"],
            "settings": {"recommendedRam": 6144}
        }));

        let written = serde_json::to_value(&pack).unwrap();

        assert_eq!(written["loader"]["type"], "neoforge");
        assert_eq!(written["base"]["provider"], "curseforge");
        assert_eq!(written["files"][0]["mode"], "once");
        assert_eq!(written["settings"]["recommendedRam"], 6144);

        let parsed: Manifest = serde_json::from_value(written).unwrap();
        assert_eq!(parsed, pack);
    }
}
