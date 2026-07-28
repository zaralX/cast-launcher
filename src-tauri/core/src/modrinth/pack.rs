use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::safe_join;
use crate::instance::LoaderType;
use crate::net::download::DownloadTask;

pub const INDEX_ENTRY: &str = "modrinth.index.json";

pub const OVERRIDES: &[&str] = &["overrides", "client-overrides"];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackIndex {
    #[serde(default)]
    pub format_version: u32,
    #[serde(default)]
    pub game: String,
    #[serde(default)]
    pub version_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub files: Vec<PackFile>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub path: String,
    #[serde(default)]
    pub hashes: BTreeMap<String, String>,
    #[serde(default)]
    pub env: Option<FileEnv>,
    #[serde(default)]
    pub downloads: Vec<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEnv {
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

impl PackFile {
    pub fn needed_on_client(&self) -> bool {
        self.env
            .as_ref()
            .and_then(|env| env.client.as_deref())
            .map(|state| state != "unsupported")
            .unwrap_or(true)
    }

    pub fn sha1(&self) -> Option<String> {
        self.hashes.get("sha1").cloned()
    }
}

impl PackIndex {
    pub fn parse(bytes: &[u8]) -> CommandResult<Self> {
        serde_json::from_slice(bytes).map_err(|e| {
            CommandError::manifest(format!("Повреждённый {INDEX_ENTRY} внутри модпака"))
                .with_details(e.to_string())
        })
    }

    pub fn minecraft_version(&self) -> CommandResult<&str> {
        self.dependencies
            .get("minecraft")
            .map(String::as_str)
            .filter(|version| !version.is_empty())
            .ok_or_else(|| CommandError::manifest("В модпаке не указана версия Minecraft"))
    }

    pub fn loader(&self) -> CommandResult<(LoaderType, Option<String>)> {
        let minecraft = self.minecraft_version()?;

        for (name, version) in &self.dependencies {
            match name.as_str() {
                "fabric-loader" => return Ok((LoaderType::Fabric, Some(version.clone()))),
                "forge" => return Ok((LoaderType::Forge, Some(forge_version(minecraft, version)))),
                "quilt-loader" => return Err(unsupported("Quilt")),
                "neoforge" => return Err(unsupported("NeoForge")),
                _ => {}
            }
        }

        Ok((LoaderType::Vanilla, None))
    }

    pub fn client_tasks(&self, minecraft_dir: &Path) -> CommandResult<Vec<DownloadTask>> {
        let mut tasks = Vec::new();

        for file in self.files.iter().filter(|file| file.needed_on_client()) {
            let destination = safe_join(minecraft_dir, &file.path)?;

            let url = file.downloads.first().ok_or_else(|| {
                CommandError::manifest(format!("В модпаке нет ссылки на файл: {}", file.path))
            })?;

            tasks.push(DownloadTask::verified(
                url.clone(),
                destination,
                file.file_size,
                file.sha1(),
            ));
        }

        Ok(tasks)
    }
}

fn forge_version(minecraft: &str, forge: &str) -> String {
    if forge.starts_with(&format!("{minecraft}-")) {
        forge.to_string()
    } else {
        format!("{minecraft}-{forge}")
    }
}

fn unsupported(loader: &str) -> CommandError {
    CommandError::manifest(format!("Модпаки на {loader} пока не поддерживаются"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn index(json: serde_json::Value) -> PackIndex {
        PackIndex::parse(&serde_json::to_vec(&json).unwrap()).unwrap()
    }

    fn fabric_pack() -> PackIndex {
        index(serde_json::json!({
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.2.3",
            "name": "Пак",
            "dependencies": {"minecraft": "1.20.1", "fabric-loader": "0.15.7"},
            "files": [
                {
                    "path": "mods/jei.jar",
                    "hashes": {"sha1": "aaa", "sha512": "bbb"},
                    "downloads": ["https://cdn.modrinth.com/jei.jar"],
                    "fileSize": 1024
                },
                {
                    "path": "mods/server-only.jar",
                    "hashes": {"sha1": "ccc"},
                    "env": {"client": "unsupported", "server": "required"},
                    "downloads": ["https://cdn.modrinth.com/server.jar"]
                }
            ]
        }))
    }

    #[test]
    fn server_only_files_are_skipped_on_the_client() {
        let tasks = fabric_pack().client_tasks(Path::new("/mc")).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].destination, PathBuf::from("/mc").join("mods").join("jei.jar"));
        assert_eq!(tasks[0].sha1.as_deref(), Some("aaa"));
        assert_eq!(tasks[0].size, Some(1024));
    }

    #[test]
    fn fabric_loader_version_comes_from_the_manifest() {
        assert_eq!(fabric_pack().loader().unwrap(), (LoaderType::Fabric, Some("0.15.7".into())));
        assert_eq!(fabric_pack().minecraft_version().unwrap(), "1.20.1");
    }

    #[test]
    fn forge_version_is_normalised_to_the_maven_form() {
        let short = index(serde_json::json!({
            "dependencies": {"minecraft": "1.20.1", "forge": "47.2.0"}
        }));
        assert_eq!(short.loader().unwrap(), (LoaderType::Forge, Some("1.20.1-47.2.0".into())));

        let full = index(serde_json::json!({
            "dependencies": {"minecraft": "1.20.1", "forge": "1.20.1-47.2.0"}
        }));
        assert_eq!(full.loader().unwrap(), (LoaderType::Forge, Some("1.20.1-47.2.0".into())));
    }

    #[test]
    fn a_pack_without_a_loader_is_plain_vanilla() {
        let vanilla = index(serde_json::json!({"dependencies": {"minecraft": "1.20.1"}}));
        assert_eq!(vanilla.loader().unwrap(), (LoaderType::Vanilla, None));
    }

    #[test]
    fn loaders_we_cannot_install_are_reported_by_name() {
        let quilt = index(serde_json::json!({
            "dependencies": {"minecraft": "1.20.1", "quilt-loader": "0.23.1"}
        }));

        assert!(quilt.loader().unwrap_err().message.contains("Quilt"));

        let neoforge = index(serde_json::json!({
            "dependencies": {"minecraft": "1.21", "neoforge": "21.0.0"}
        }));

        assert!(neoforge.loader().unwrap_err().message.contains("NeoForge"));
    }

    #[test]
    fn a_pack_without_a_minecraft_version_is_rejected() {
        let broken = index(serde_json::json!({"dependencies": {"fabric-loader": "0.15.7"}}));

        assert!(broken.minecraft_version().is_err());
        assert!(broken.loader().is_err());
    }

    #[test]
    fn files_cannot_escape_the_instance_directory() {
        let evil = index(serde_json::json!({
            "dependencies": {"minecraft": "1.20.1"},
            "files": [{"path": "../../../config.json", "downloads": ["https://cdn/evil"]}]
        }));

        assert!(evil.client_tasks(Path::new("/mc")).is_err());
    }

    #[test]
    fn a_file_without_a_download_link_is_an_error() {
        let broken = index(serde_json::json!({
            "dependencies": {"minecraft": "1.20.1"},
            "files": [{"path": "mods/a.jar", "downloads": []}]
        }));

        assert!(broken.client_tasks(Path::new("/mc")).is_err());
    }

    #[test]
    fn broken_json_is_reported_as_a_manifest_problem() {
        let error = PackIndex::parse(b"{ not json").unwrap_err();
        assert_eq!(error.code, "MANIFEST_INVALID");
    }
}
