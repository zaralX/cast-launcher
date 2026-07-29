use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use zip::ZipArchive;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{ensure_dir, safe_join, write_json_atomic};
use crate::meta::forge::InstalledLoader;
use crate::mojang::maven::Gradle;
use crate::mojang::profile::{resolve_artifact, resolve_libraries, ResolvedArtifact};
use crate::mojang::rules::RuntimeContext;
use crate::mojang::version::{Library, VersionPackage};
use crate::net::download::DownloadTask;
use crate::paths::{LauncherPaths, LoaderPaths};

const INSTALL_PROFILE: &str = "install_profile.json";
const DEFAULT_VERSION_JSON: &str = "/version.json";
const BUNDLED_PREFIX: &str = "maven/";
const CLIENT: &str = "client";
const PATCHED: &str = "PATCHED";

#[derive(Debug, Clone)]
pub struct Processor {
    pub jar: String,
    pub classpath: Vec<String>,
    pub args: Vec<String>,
    pub outputs: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone)]
struct Bundled {
    entry: String,
    path: String,
    size: Option<u64>,
}

#[derive(Debug)]
pub struct Installer {
    jar: PathBuf,
    minecraft: String,
    version_json: Value,
    package: VersionPackage,
    setup_libraries: Vec<Library>,
    bundled: Vec<Bundled>,
    produced: HashSet<String>,
    data: BTreeMap<String, String>,
    processors: Vec<Processor>,
}

impl Installer {
    pub async fn open(jar: PathBuf) -> CommandResult<Self> {
        tokio::task::spawn_blocking(move || parse(jar))
            .await
            .map_err(|e| CommandError::task_panicked("чтение установщика Forge", e))?
    }

    pub fn minecraft_version(&self) -> &str {
        &self.minecraft
    }

    pub fn version_id(&self) -> &str {
        &self.package.id
    }

    pub fn processors(&self) -> &[Processor] {
        &self.processors
    }

    pub fn data(&self) -> &BTreeMap<String, String> {
        &self.data
    }

    pub fn patched_client(&self) -> Option<&str> {
        let value = self.data.get(PATCHED)?.as_str();

        value.strip_prefix('[')?.strip_suffix(']')
    }

    pub async fn unpack(&self, paths: &LauncherPaths) -> CommandResult<usize> {
        let pending: Vec<Bundled> = self
            .bundled
            .iter()
            .filter(|bundled| !is_intact(&paths.library(&bundled.path), bundled.size))
            .cloned()
            .collect();

        if pending.is_empty() {
            return Ok(0);
        }

        let jar = self.jar.clone();
        let libraries = paths.libraries();

        tokio::task::spawn_blocking(move || unpack_blocking(&jar, &libraries, &pending))
            .await
            .map_err(|e| CommandError::task_panicked("распаковка файлов Forge", e))?
    }

    pub fn downloads(&self, paths: &LauncherPaths, ctx: &RuntimeContext) -> Vec<DownloadTask> {
        let mut tasks = Vec::new();
        let mut seen = HashSet::new();

        for artifact in self.artifacts(ctx) {
            if self.produced.contains(&artifact.path) || !seen.insert(artifact.path.clone()) {
                continue;
            }

            let Some(url) = &artifact.url else { continue };

            tasks.push(DownloadTask::verified(
                url.clone(),
                paths.library(&artifact.path),
                artifact.size,
                artifact.sha1.clone(),
            ));
        }

        tasks
    }

    pub fn missing(&self, paths: &LauncherPaths, ctx: &RuntimeContext) -> Vec<String> {
        let mut missing = Vec::new();
        let mut seen = HashSet::new();

        if let Some(patched) = self.patched_client().and_then(|c| Gradle::parse(c).ok()) {
            let path = patched.path();

            if seen.insert(path.clone()) && !paths.library(&path).is_file() {
                missing.push(path);
            }
        }

        for library in resolve_libraries(&self.package.libraries, ctx) {
            for artifact in library.artifacts() {
                if !seen.insert(artifact.path.clone()) {
                    continue;
                }

                if !paths.library(&artifact.path).is_file() {
                    missing.push(artifact.path.clone());
                }
            }
        }

        missing
    }

    pub async fn save(&self, cache: &LoaderPaths) -> CommandResult<()> {
        ensure_dir(cache.root()).await?;
        write_json_atomic(&cache.client_json(), &self.version_json).await?;

        let installed = InstalledLoader {
            minecraft_version: self.minecraft.clone(),
            patched_client: self.patched_client().map(str::to_string),
        };

        write_json_atomic(&cache.installed_json(), &installed).await
    }

    fn artifacts(&self, ctx: &RuntimeContext) -> Vec<ResolvedArtifact> {
        let setup = resolve_libraries(&self.setup_libraries, ctx);
        let game = resolve_libraries(&self.package.libraries, ctx);

        setup
            .iter()
            .chain(game.iter())
            .flat_map(|library| library.artifacts())
            .cloned()
            .collect()
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawProfile {
    #[serde(default)]
    install: Option<RawLegacyInstall>,
    #[serde(default, rename = "versionInfo")]
    version_info: Option<Value>,
    #[serde(default)]
    json: Option<String>,
    #[serde(default)]
    minecraft: Option<String>,
    #[serde(default)]
    data: BTreeMap<String, RawSided>,
    #[serde(default)]
    processors: Vec<RawProcessor>,
    #[serde(default)]
    libraries: Vec<Library>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLegacyInstall {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    minecraft: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSided {
    #[serde(default)]
    client: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawProcessor {
    #[serde(default)]
    sides: Option<Vec<String>>,
    jar: String,
    #[serde(default)]
    classpath: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    outputs: BTreeMap<String, Option<String>>,
}

fn parse(jar: PathBuf) -> CommandResult<Installer> {
    let mut archive = open_zip(&jar)?;
    let names: HashSet<String> = archive.file_names().map(str::to_string).collect();

    let raw: RawProfile = serde_json::from_slice(&read_entry(&mut archive, INSTALL_PROFILE)?)
        .map_err(|e| {
            CommandError::manifest("Установщик Forge содержит нечитаемый install_profile.json")
                .with_details(e.to_string())
        })?;

    match (&raw.install, &raw.version_info) {
        (Some(_), Some(_)) => parse_legacy(jar, raw),
        _ => parse_modern(jar, raw, &mut archive, &names),
    }
}

fn parse_legacy(jar: PathBuf, raw: RawProfile) -> CommandResult<Installer> {
    let install = raw.install.expect("install проверен вызывающим кодом");
    let version_json = raw.version_info.expect("versionInfo проверен вызывающим кодом");

    let coordinate = install.path.as_deref().ok_or_else(|| {
        CommandError::forge("Установщик Forge не указывает координату universal-архива")
    })?;

    let entry = install
        .file_path
        .as_deref()
        .map(|path| path.trim_start_matches('/').to_string())
        .filter(|path| !path.is_empty())
        .ok_or_else(|| {
            CommandError::forge(
                "Эта версия Forge слишком старая: установщик не содержит universal-архива",
            )
        })?;

    let path = Gradle::parse(coordinate)?.path();
    let package = package_of(&version_json)?;

    let minecraft = install
        .minecraft
        .clone()
        .or_else(|| inherits_from(&version_json))
        .ok_or_else(|| CommandError::forge("Установщик Forge не указывает версию Minecraft"))?;

    Ok(Installer {
        jar,
        minecraft,
        version_json,
        package,
        setup_libraries: Vec::new(),
        bundled: vec![Bundled {
            entry,
            path: path.clone(),
            size: None,
        }],
        produced: HashSet::from([path]),
        data: BTreeMap::new(),
        processors: Vec::new(),
    })
}

fn parse_modern(
    jar: PathBuf,
    raw: RawProfile,
    archive: &mut ZipArchive<File>,
    names: &HashSet<String>,
) -> CommandResult<Installer> {
    let entry = raw
        .json
        .as_deref()
        .unwrap_or(DEFAULT_VERSION_JSON)
        .trim_start_matches('/')
        .to_string();

    let version_json: Value = serde_json::from_slice(&read_entry(archive, &entry)?).map_err(|e| {
        CommandError::manifest(format!("Установщик Forge содержит нечитаемый {entry}"))
            .with_details(e.to_string())
    })?;

    let package = package_of(&version_json)?;

    let minecraft = raw
        .minecraft
        .clone()
        .or_else(|| inherits_from(&version_json))
        .ok_or_else(|| CommandError::forge("Установщик Forge не указывает версию Minecraft"))?;

    let mut bundled = Vec::new();
    let mut produced = HashSet::new();

    for library in raw.libraries.iter().chain(package.libraries.iter()) {
        let Some(artifact) = resolve_artifact(library) else { continue };

        if produced.contains(&artifact.path) {
            continue;
        }

        let entry = format!("{BUNDLED_PREFIX}{}", artifact.path);

        if names.contains(&entry) {
            bundled.push(Bundled {
                entry,
                path: artifact.path.clone(),
                size: artifact.size,
            });
            produced.insert(artifact.path);
        } else if artifact.url.is_none() {
            produced.insert(artifact.path);
        }
    }

    let data = raw
        .data
        .into_iter()
        .filter_map(|(key, sided)| sided.client.map(|value| (key, value)))
        .collect();

    let processors = raw
        .processors
        .into_iter()
        .filter(runs_on_client)
        .map(|processor| Processor {
            jar: processor.jar,
            classpath: processor.classpath,
            args: processor.args,
            outputs: processor.outputs,
        })
        .collect();

    Ok(Installer {
        jar,
        minecraft,
        version_json,
        package,
        setup_libraries: raw.libraries,
        bundled,
        produced,
        data,
        processors,
    })
}

fn runs_on_client(processor: &RawProcessor) -> bool {
    match &processor.sides {
        Some(sides) => sides.iter().any(|side| side == CLIENT),
        None => true,
    }
}

fn package_of(version_json: &Value) -> CommandResult<VersionPackage> {
    serde_json::from_value(version_json.clone()).map_err(|e| {
        CommandError::manifest("Установщик Forge содержит нечитаемый манифест версии")
            .with_details(e.to_string())
    })
}

fn inherits_from(version_json: &Value) -> Option<String> {
    version_json
        .get("inheritsFrom")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn is_intact(path: &Path, size: Option<u64>) -> bool {
    match (std::fs::metadata(path), size) {
        (Ok(meta), Some(size)) => meta.is_file() && meta.len() == size,
        (Ok(meta), None) => meta.is_file(),
        (Err(_), _) => false,
    }
}

fn unpack_blocking(jar: &Path, libraries: &Path, pending: &[Bundled]) -> CommandResult<usize> {
    let mut archive = open_zip(jar)?;
    let mut unpacked = 0;

    for bundled in pending {
        let bytes = read_entry(&mut archive, &bundled.entry)?;
        let target = safe_join(libraries, &bundled.path)?;
        let temp = target.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CommandError::io("Не удалось создать каталог", parent, e))?;
        }

        std::fs::write(&temp, &bytes)
            .map_err(|e| CommandError::io("Не удалось распаковать файл Forge", &temp, e))?;

        if let Err(error) = std::fs::rename(&temp, &target) {
            let _ = std::fs::remove_file(&temp);
            return Err(CommandError::io("Не удалось сохранить файл Forge", &target, error));
        }

        unpacked += 1;
    }

    Ok(unpacked)
}

fn open_zip(jar: &Path) -> CommandResult<ZipArchive<File>> {
    let file = File::open(jar)
        .map_err(|e| CommandError::io("Не удалось открыть установщик Forge", jar, e))?;

    ZipArchive::new(file).map_err(|e| {
        CommandError::archive(format!("Повреждённый установщик Forge: {}", jar.display()))
            .with_details(e.to_string())
    })
}

fn read_entry(archive: &mut ZipArchive<File>, name: &str) -> CommandResult<Vec<u8>> {
    let mut entry = archive.by_name(name).map_err(|e| {
        CommandError::archive(format!("В установщике Forge нет файла {name}"))
            .with_details(e.to_string())
    })?;

    let mut bytes = Vec::with_capacity(entry.size() as usize);

    entry.read_to_end(&mut bytes).map_err(|e| {
        CommandError::archive(format!("Не удалось прочитать {name} из установщика Forge"))
            .with_details(e.to_string())
    })?;

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mojang::rules::MojangOs;
    use serde_json::json;
    use std::io::Write;

    fn ctx() -> RuntimeContext {
        RuntimeContext {
            os: MojangOs::Windows,
            arch: "x86_64".into(),
            os_version: "10.0".into(),
        }
    }

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-forge-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_installer(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();

        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }

        writer.finish().unwrap();
    }

    fn legacy_profile() -> Value {
        json!({
            "install": {
                "path": "net.minecraftforge:forge:1.7.10-10.13.4.1614-1.7.10",
                "filePath": "forge-1.7.10-10.13.4.1614-1.7.10-universal.jar",
                "minecraft": "1.7.10"
            },
            "versionInfo": {
                "id": "1.7.10-Forge10.13.4.1614-1.7.10",
                "inheritsFrom": "1.7.10",
                "mainClass": "net.minecraft.launchwrapper.Launch",
                "minecraftArguments": "--username ${auth_player_name} --tweakClass cpw.mods.fml.common.launcher.FMLTweaker",
                "libraries": [
                    { "name": "net.minecraftforge:forge:1.7.10-10.13.4.1614-1.7.10", "url": "https://maven.minecraftforge.net/" },
                    { "name": "net.minecraft:launchwrapper:1.12" }
                ]
            }
        })
    }

    fn modern_profile() -> Value {
        json!({
            "spec": 1,
            "profile": "forge",
            "version": "1.21.11-forge-61.1.14",
            "json": "/version.json",
            "path": "net.minecraftforge:forge:1.21.11-61.1.14:shim",
            "minecraft": "1.21.11",
            "data": {
                "BINPATCH": { "client": "/data/client.lzma", "server": "/data/server.lzma" },
                "PATCHED": {
                    "client": "[net.minecraftforge:forge:1.21.11-61.1.14:client]",
                    "server": "[net.minecraftforge:forge:1.21.11-61.1.14:server]"
                }
            },
            "processors": [
                { "sides": ["server"], "jar": "net.minecraftforge:installertools:1.4.3", "args": ["--task", "BUNDLER_EXTRACT"] },
                { "jar": "net.minecraftforge:binarypatcher:1.3.1", "classpath": ["org.ow2.asm:asm:9.7"], "args": ["--output", "{PATCHED}"], "outputs": { "{PATCHED}": "{PATCHED_SHA}" } }
            ],
            "libraries": [
                {
                    "name": "net.minecraftforge:binarypatcher:1.3.1",
                    "downloads": { "artifact": {
                        "path": "net/minecraftforge/binarypatcher/1.3.1/binarypatcher-1.3.1.jar",
                        "url": "https://maven.minecraftforge.net/net/minecraftforge/binarypatcher/1.3.1/binarypatcher-1.3.1.jar",
                        "sha1": "aaa", "size": 10
                    }}
                },
                {
                    "name": "net.minecraftforge:forge:1.21.11-61.1.14:universal",
                    "downloads": { "artifact": {
                        "path": "net/minecraftforge/forge/1.21.11-61.1.14/forge-1.21.11-61.1.14-universal.jar",
                        "url": "", "sha1": "bbb", "size": 9
                    }}
                }
            ]
        })
    }

    fn modern_version() -> Value {
        json!({
            "id": "1.21.11-forge-61.1.14",
            "inheritsFrom": "1.21.11",
            "mainClass": "net.minecraftforge.bootstrap.ForgeBootstrap",
            "arguments": { "game": ["--launchTarget", "forge_client"], "jvm": [] },
            "libraries": [
                {
                    "name": "net.minecraftforge:forge:1.21.11-61.1.14:client",
                    "downloads": { "artifact": {
                        "path": "net/minecraftforge/forge/1.21.11-61.1.14/forge-1.21.11-61.1.14-client.jar",
                        "url": "", "sha1": "ccc", "size": 30
                    }}
                },
                {
                    "name": "net.minecraftforge:forge:1.21.11-61.1.14:universal",
                    "downloads": { "artifact": {
                        "path": "net/minecraftforge/forge/1.21.11-61.1.14/forge-1.21.11-61.1.14-universal.jar",
                        "url": "", "sha1": "bbb", "size": 9
                    }}
                },
                {
                    "name": "org.ow2.asm:asm:9.7",
                    "downloads": { "artifact": {
                        "path": "org/ow2/asm/asm/9.7/asm-9.7.jar",
                        "url": "https://maven.minecraftforge.net/org/ow2/asm/asm/9.7/asm-9.7.jar",
                        "sha1": "ddd", "size": 40
                    }}
                }
            ]
        })
    }

    #[tokio::test]
    async fn legacy_installer_provides_the_universal_jar_itself() {
        let dir = scratch();
        let jar = dir.join("installer.jar");

        write_installer(&jar, &[
            ("install_profile.json", legacy_profile().to_string().as_bytes()),
            ("forge-1.7.10-10.13.4.1614-1.7.10-universal.jar", b"universal"),
        ]);

        let installer = Installer::open(jar).await.unwrap();

        assert_eq!(installer.minecraft_version(), "1.7.10");
        assert!(installer.processors().is_empty());

        let paths = LauncherPaths::new(dir.clone(), None);
        assert_eq!(installer.unpack(&paths).await.unwrap(), 1);

        let universal = paths.library(
            "net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10.jar",
        );
        assert_eq!(std::fs::read(&universal).unwrap(), b"universal");

        assert_eq!(installer.unpack(&paths).await.unwrap(), 0);

        let urls: Vec<String> = installer
            .downloads(&paths, &ctx())
            .into_iter()
            .map(|task| task.url)
            .collect();

        assert_eq!(urls, vec!["https://libraries.minecraft.net/net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn modern_installer_separates_bundled_produced_and_downloaded_files() {
        let dir = scratch();
        let jar = dir.join("installer.jar");

        write_installer(&jar, &[
            ("install_profile.json", modern_profile().to_string().as_bytes()),
            ("version.json", modern_version().to_string().as_bytes()),
            ("data/client.lzma", b"patch"),
            (
                "maven/net/minecraftforge/forge/1.21.11-61.1.14/forge-1.21.11-61.1.14-universal.jar",
                b"universal",
            ),
        ]);

        let installer = Installer::open(jar).await.unwrap();

        assert_eq!(installer.minecraft_version(), "1.21.11");
        assert_eq!(installer.version_id(), "1.21.11-forge-61.1.14");
        assert_eq!(installer.processors().len(), 1, "серверные процессоры отброшены");
        assert_eq!(installer.processors()[0].jar, "net.minecraftforge:binarypatcher:1.3.1");
        assert_eq!(installer.data().get("BINPATCH").unwrap(), "/data/client.lzma");

        let paths = LauncherPaths::new(dir.clone(), None);
        let universal = paths
            .library("net/minecraftforge/forge/1.21.11-61.1.14/forge-1.21.11-61.1.14-universal.jar");

        assert_eq!(installer.unpack(&paths).await.unwrap(), 1);
        assert_eq!(installer.unpack(&paths).await.unwrap(), 0);

        std::fs::write(&universal, b"cut").unwrap();
        assert_eq!(installer.unpack(&paths).await.unwrap(), 1, "битый размер перекачивается");
        assert_eq!(std::fs::read(&universal).unwrap(), b"universal");

        let urls: Vec<String> = installer
            .downloads(&paths, &ctx())
            .into_iter()
            .map(|task| task.url)
            .collect();

        assert_eq!(urls, vec![
            "https://maven.minecraftforge.net/net/minecraftforge/binarypatcher/1.3.1/binarypatcher-1.3.1.jar",
            "https://maven.minecraftforge.net/org/ow2/asm/asm/9.7/asm-9.7.jar",
        ]);

        let missing = installer.missing(&paths, &ctx());
        assert_eq!(missing, vec![
            "net/minecraftforge/forge/1.21.11-61.1.14/forge-1.21.11-61.1.14-client.jar",
            "org/ow2/asm/asm/9.7/asm-9.7.jar",
        ]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn the_saved_manifest_is_the_one_from_the_installer() {
        let dir = scratch();
        let jar = dir.join("installer.jar");

        write_installer(&jar, &[
            ("install_profile.json", modern_profile().to_string().as_bytes()),
            ("version.json", modern_version().to_string().as_bytes()),
        ]);

        let installer = Installer::open(jar).await.unwrap();
        let paths = LauncherPaths::new(dir.clone(), None);
        let cache = paths.loader_cache("forge", "1.21.11-61.1.14");

        installer.save(&cache).await.unwrap();

        let saved: Value = crate::fs_util::read_json(&cache.client_json()).await.unwrap();
        assert_eq!(saved, modern_version());

        let installed: InstalledLoader = crate::fs_util::read_json(&cache.installed_json()).await.unwrap();
        assert_eq!(installed.minecraft_version, "1.21.11");
        assert_eq!(
            installed.patched_client.as_deref(),
            Some("net.minecraftforge:forge:1.21.11-61.1.14:client")
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_neoforge_installer_is_read_the_same_way_as_a_forge_one() {
        let dir = scratch();
        let jar = dir.join("installer.jar");

        let profile = json!({
            "spec": 1,
            "profile": "NeoForge",
            "version": "neoforge-26.1.2.86",
            "json": "/version.json",
            "minecraft": "26.1.2",
            "data": {
                "BINPATCH": { "client": "/data/client.lzma", "server": "/data/client.lzma" },
                "PATCHED": {
                    "client": "[net.neoforged:minecraft-client-patched:26.1.2.86]",
                    "server": "[net.neoforged:minecraft-server-patched:26.1.2.86]"
                }
            },
            "processors": [
                {
                    "sides": ["server"],
                    "jar": "net.neoforged.installertools:installertools:4.0.12:fatjar",
                    "args": ["--task", "EXTRACT_FILES"]
                },
                {
                    "jar": "net.neoforged.installertools:installertools:4.0.12:fatjar",
                    "args": ["--task", "PROCESS_MINECRAFT_JAR", "--output", "{PATCHED}"]
                }
            ],
            "libraries": [{
                "name": "net.neoforged.installertools:installertools:4.0.12:fatjar",
                "downloads": { "artifact": {
                    "path": "net/neoforged/installertools/installertools/4.0.12/installertools-4.0.12-fatjar.jar",
                    "url": "https://maven.neoforged.net/releases/net/neoforged/installertools/installertools/4.0.12/installertools-4.0.12-fatjar.jar",
                    "sha1": "aaa", "size": 10
                }}
            }]
        });

        let version = json!({
            "id": "neoforge-26.1.2.86",
            "inheritsFrom": "26.1.2",
            "mainClass": "net.neoforged.fml.startup.Client",
            "arguments": { "game": ["--fml.neoForgeVersion", "26.1.2.86"], "jvm": [] },
            "libraries": []
        });

        write_installer(&jar, &[
            ("install_profile.json", profile.to_string().as_bytes()),
            ("version.json", version.to_string().as_bytes()),
        ]);

        let installer = Installer::open(jar).await.unwrap();

        assert_eq!(installer.minecraft_version(), "26.1.2");
        assert_eq!(installer.version_id(), "neoforge-26.1.2.86");
        assert_eq!(installer.processors().len(), 1, "серверные процессоры отброшены");
        assert_eq!(
            installer.patched_client(),
            Some("net.neoforged:minecraft-client-patched:26.1.2.86")
        );

        let paths = LauncherPaths::new(dir.clone(), None);

        // Собранного клиента нет ни в манифесте версии, ни в outputs процессоров —
        // без отдельной проверки поломка вылезла бы только при запуске игры.
        assert_eq!(
            installer.missing(&paths, &ctx()),
            vec![
                "net/neoforged/minecraft-client-patched/26.1.2.86/minecraft-client-patched-26.1.2.86.jar"
            ]
        );

        let cache = paths.loader_cache("neoforge", "26.1.2.86");

        installer.save(&cache).await.unwrap();

        let installed: InstalledLoader = crate::fs_util::read_json(&cache.installed_json()).await.unwrap();
        assert_eq!(
            installed.patched_client.as_deref(),
            Some("net.neoforged:minecraft-client-patched:26.1.2.86")
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_installer_without_processors_builds_nothing_to_point_at() {
        let dir = scratch();
        let jar = dir.join("installer.jar");

        write_installer(&jar, &[
            ("install_profile.json", legacy_profile().to_string().as_bytes()),
            ("forge-1.7.10-10.13.4.1614-1.7.10-universal.jar", b"universal"),
        ]);

        assert!(Installer::open(jar).await.unwrap().patched_client().is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_broken_installer_is_reported_as_such() {
        let dir = scratch();

        let empty = dir.join("empty.jar");
        std::fs::write(&empty, b"not a zip").unwrap();
        assert_eq!(Installer::open(empty).await.unwrap_err().code, "ARCHIVE_INVALID");

        let without_profile = dir.join("bare.jar");
        write_installer(&without_profile, &[("readme.txt", b"hi")]);
        assert_eq!(
            Installer::open(without_profile).await.unwrap_err().code,
            "ARCHIVE_INVALID"
        );

        let broken = dir.join("broken.jar");
        write_installer(&broken, &[("install_profile.json", b"{ not json")]);
        assert_eq!(Installer::open(broken).await.unwrap_err().code, "MANIFEST_INVALID");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_produced_artifact_is_never_downloaded_even_with_a_dead_url() {
        let dir = scratch();
        let jar = dir.join("installer.jar");

        let profile = json!({
            "install": {
                "path": "net.minecraftforge:forge:1.12.2-14.23.5.2859",
                "filePath": "/forge-universal.jar",
                "minecraft": "1.12.2"
            },
            "versionInfo": {
                "id": "1.12.2-forge",
                "mainClass": "net.minecraft.launchwrapper.Launch",
                "libraries": [
                    { "name": "net.minecraftforge:forge:1.12.2-14.23.5.2859", "url": "https://maven.minecraftforge.net/" }
                ]
            }
        });

        write_installer(&jar, &[
            ("install_profile.json", profile.to_string().as_bytes()),
            ("forge-universal.jar", b"universal"),
        ]);

        let installer = Installer::open(jar).await.unwrap();
        let paths = LauncherPaths::new(dir.clone(), None);

        assert!(installer.downloads(&paths, &ctx()).is_empty());

        installer.unpack(&paths).await.unwrap();
        assert!(installer.missing(&paths, &ctx()).is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
