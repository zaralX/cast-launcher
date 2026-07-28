use serde::Deserialize;

use crate::error::{CommandError, CommandResult};
use crate::instance::Instance;
use crate::mojang::maven::Gradle;
use crate::mojang::profile::{
    resolve_libraries, ResolvedArtifact, ResolvedLibrary, ResolvedProfile,
};
use crate::mojang::rules::RuntimeContext;
use crate::mojang::version::VersionPackage;
use crate::net::meta_cache::MetaCache;
use crate::paths::LauncherPaths;

const FABRIC_META: &str = "https://meta.fabricmc.net/v2/versions/loader";
const FABRIC_MAVEN: &str = "https://maven.fabricmc.net/";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoader {
    pub loader: FabricMaven,
    pub intermediary: FabricMaven,
    pub launcher_meta: FabricLauncherMeta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FabricMaven {
    pub maven: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricLauncherMeta {
    #[serde(default)]
    pub libraries: FabricLibraries,
    pub main_class: FabricMainClass,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FabricLibraries {
    #[serde(default)]
    pub common: Vec<FabricLibrary>,
    #[serde(default)]
    pub client: Vec<FabricLibrary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FabricLibrary {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FabricMainClass {
    Split { client: String },
    Single(String),
}

impl FabricMainClass {
    pub fn client(&self) -> &str {
        match self {
            Self::Split { client } => client,
            Self::Single(value) => value,
        }
    }
}

pub async fn loader(
    meta: &MetaCache,
    minecraft_version: &str,
    loader_version: &str,
) -> CommandResult<FabricLoader> {
    let not_found = || {
        CommandError::version_not_found(format!(
            "Fabric {loader_version} недоступен для Minecraft {minecraft_version}"
        ))
    };

    if loader_version.is_empty() || loader_version == "latest" {
        let candidates: Vec<FabricLoader> = meta
            .fetch_json(&format!("{FABRIC_META}/{minecraft_version}"))
            .await?;

        return candidates.into_iter().next().ok_or_else(not_found);
    }

    meta.fetch_json(&format!("{FABRIC_META}/{minecraft_version}/{loader_version}"))
        .await
}

pub fn libraries(loader: &FabricLoader) -> CommandResult<Vec<ResolvedLibrary>> {
    let mut libraries = Vec::new();

    for coordinate in [&loader.loader.maven, &loader.intermediary.maven] {
        libraries.push(library(coordinate, None, None, None)?);
    }

    for entry in loader
        .launcher_meta
        .libraries
        .common
        .iter()
        .chain(&loader.launcher_meta.libraries.client)
    {
        libraries.push(library(
            &entry.name,
            entry.url.as_deref(),
            entry.sha1.clone(),
            entry.size,
        )?);
    }

    Ok(libraries)
}

fn library(
    coordinate: &str,
    repository: Option<&str>,
    sha1: Option<String>,
    size: Option<u64>,
) -> CommandResult<ResolvedLibrary> {
    let gradle = Gradle::parse(coordinate)?;
    let repository = repository.unwrap_or(FABRIC_MAVEN);

    Ok(ResolvedLibrary {
        name: Some(coordinate.to_string()),
        artifact: Some(ResolvedArtifact {
            path: gradle.path(),
            url: Some(gradle.url(repository)),
            sha1,
            size,
        }),
        native: None,
    })
}

pub fn profile(
    paths: &LauncherPaths,
    instance: &Instance,
    vanilla: &VersionPackage,
    loader: &FabricLoader,
    ctx: &RuntimeContext,
) -> CommandResult<ResolvedProfile> {
    let mut profile = super::vanilla::profile(paths, instance, vanilla, ctx)?;

    let mut merged = libraries(loader)?;
    merged.extend(resolve_libraries(&vanilla.libraries, ctx));

    profile.version_type = "Fabric".into();
    profile.main_class = loader.launcher_meta.main_class.client().to_string();
    profile.libraries = merged;

    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> FabricLoader {
        serde_json::from_value(json!({
            "loader": { "maven": "net.fabricmc:fabric-loader:0.15.7" },
            "intermediary": { "maven": "net.fabricmc:intermediary:1.20.1" },
            "launcherMeta": {
                "mainClass": { "client": "net.fabricmc.loader.impl.launch.knot.KnotClient" },
                "libraries": {
                    "common": [{ "name": "org.ow2.asm:asm:9.6", "url": "https://maven.example/", "sha1": "abc", "size": 10 }],
                    "client": []
                }
            }
        }))
        .unwrap()
    }

    #[test]
    fn loader_and_intermediary_come_first() {
        let libraries = libraries(&sample()).unwrap();

        assert_eq!(libraries.len(), 3);
        assert_eq!(
            libraries[0].artifact.as_ref().unwrap().path,
            "net/fabricmc/fabric-loader/0.15.7/fabric-loader-0.15.7.jar"
        );
        assert_eq!(
            libraries[1].artifact.as_ref().unwrap().path,
            "net/fabricmc/intermediary/1.20.1/intermediary-1.20.1.jar"
        );
    }

    #[test]
    fn library_repository_defaults_to_fabric_maven() {
        let libraries = libraries(&sample()).unwrap();

        assert!(libraries[0]
            .artifact
            .as_ref()
            .unwrap()
            .url
            .as_ref()
            .unwrap()
            .starts_with(FABRIC_MAVEN));

        assert!(libraries[2]
            .artifact
            .as_ref()
            .unwrap()
            .url
            .as_ref()
            .unwrap()
            .starts_with("https://maven.example/"));
    }

    #[test]
    fn main_class_accepts_both_shapes() {
        let split: FabricMainClass = serde_json::from_value(json!({ "client": "A", "server": "B" })).unwrap();
        let single: FabricMainClass = serde_json::from_value(json!("A")).unwrap();

        assert_eq!(split.client(), "A");
        assert_eq!(single.client(), "A");
    }
}
