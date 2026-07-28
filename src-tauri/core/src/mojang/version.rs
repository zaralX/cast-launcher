use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::mojang::rules::Rule;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionPackage {
    pub id: String,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub downloads: Option<ClientDownloads>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub java_version: Option<JavaVersionSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientDownloads {
    #[serde(default)]
    pub client: Option<MojangArtifact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MojangArtifact {
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub total_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersionSpec {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<LibraryArtifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryArtifact {
    pub path: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional {
        #[serde(default)]
        rules: Option<Vec<Rule>>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    One(String),
    Many(Vec<String>),
}

impl ArgumentValue {
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        match self {
            Self::One(value) => std::slice::from_ref(value).iter(),
            Self::Many(values) => values.iter(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    #[serde(default)]
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    #[serde(default)]
    pub size: Option<u64>,
}

impl AssetObject {
    pub fn shard(&self) -> &str {
        &self.hash[..2.min(self.hash.len())]
    }

    pub fn url(&self) -> String {
        format!(
            "https://resources.download.minecraft.net/{}/{}",
            self.shard(),
            self.hash
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    #[serde(default)]
    pub latest: LatestVersions,
    #[serde(default)]
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LatestVersions {
    #[serde(default)]
    pub release: Option<String>,
    #[serde(default)]
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub id: String,
    pub url: String,
    #[serde(default, rename = "type")]
    pub release_type: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
}

impl VersionManifest {
    pub fn find(&self, id: &str) -> Option<&VersionEntry> {
        self.versions.iter().find(|entry| entry.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn arguments_accept_both_shapes() {
        let arguments: Arguments = serde_json::from_value(json!({
            "game": ["--username", "${auth_player_name}"],
            "jvm": [
                { "rules": [{ "action": "allow", "os": { "name": "osx" } }], "value": "-XstartOnFirstThread" },
                { "rules": [{ "action": "allow", "os": { "name": "windows" } }], "value": ["-Dos.name=Windows 10", "-Dos.version=10.0"] }
            ]
        }))
        .unwrap();

        assert!(matches!(arguments.game[0], Argument::Plain(_)));
        assert_eq!(arguments.jvm.len(), 2);

        let Argument::Conditional { value, .. } = &arguments.jvm[1] else {
            panic!("ожидался условный аргумент");
        };
        assert_eq!(value.iter().count(), 2);
    }

    #[test]
    fn unknown_manifest_fields_are_ignored() {
        let package: VersionPackage = serde_json::from_value(json!({
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "brandNewFieldFromMojang": { "nested": true },
            "libraries": []
        }))
        .unwrap();

        assert_eq!(package.id, "1.20.1");
        assert!(package.asset_index.is_none());
    }

    #[test]
    fn asset_objects_expose_shard_and_url() {
        let object = AssetObject {
            hash: "abcdef0123".into(),
            size: Some(10),
        };

        assert_eq!(object.shard(), "ab");
        assert_eq!(
            object.url(),
            "https://resources.download.minecraft.net/ab/abcdef0123"
        );
    }
}
