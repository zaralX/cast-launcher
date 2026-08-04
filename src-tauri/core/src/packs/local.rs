use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;
use zip::ZipArchive;

use crate::curseforge::pack::{Manifest, MANIFEST_ENTRY};
use crate::error::{CommandError, CommandResult};
use crate::import::prism::{self, CONFIG_FILE, PACK_FILE};
use crate::import::ScannedInstance;
use crate::instance::{InstanceSettings, LoaderType, LocalPackKind};
use crate::modrinth::pack::{PackIndex, INDEX_ENTRY};
use crate::packs::ResolvedPack;

const GAME_DIRS: [&str; 2] = [".minecraft", "minecraft"];

const MAX_MANIFEST: u64 = 64 * 1024 * 1024;

const MAX_ROOTS: usize = 16;

pub const EXTENSIONS: [&str; 2] = ["mrpack", "zip"];

#[derive(Debug)]
pub struct Opened {
    root: String,
    contents: Contents,
}

#[derive(Debug)]
enum Contents {
    Modrinth(Box<PackIndex>),
    CurseForge(Box<Manifest>),
    MultiMc {
        scanned: Box<ScannedInstance>,
        game_dir: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPack {
    pub kind: LocalPackKind,
    pub kind_label: &'static str,
    pub path: String,
    pub file_name: String,
    pub size: u64,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub minecraft_version: String,
    pub loader: Option<LoaderType>,
    pub loader_version: Option<String>,
    pub loader_label: String,
    pub files: usize,
    pub settings: InstanceSettings,
    pub blocked: Option<String>,
}

impl LocalPack {
    pub fn is_importable(&self) -> bool {
        self.blocked.is_none()
    }
}

pub async fn inspect(path: &Path) -> CommandResult<LocalPack> {
    let path = path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let opened = open_blocking(&path)?;
        Ok(opened.preview(&path))
    })
    .await
    .map_err(|e| CommandError::task_panicked("чтение архива модпака", e))?
}

pub async fn resolve(path: &Path, minecraft_dir: &Path) -> CommandResult<ResolvedPack> {
    let opened = {
        let path = path.to_path_buf();

        tokio::task::spawn_blocking(move || open_blocking(&path))
            .await
            .map_err(|e| CommandError::task_panicked("чтение архива модпака", e))??
    };

    opened.resolve(minecraft_dir).await
}

impl Opened {
    pub fn kind(&self) -> LocalPackKind {
        match &self.contents {
            Contents::Modrinth(_) => LocalPackKind::Modrinth,
            Contents::CurseForge(_) => LocalPackKind::CurseForge,
            Contents::MultiMc { .. } => LocalPackKind::MultiMc,
        }
    }

    fn preview(&self, path: &Path) -> LocalPack {
        let kind = self.kind();
        let file_name = file_name(path);

        let mut pack = LocalPack {
            kind,
            kind_label: kind.label(),
            path: path.display().to_string(),
            name: stem(&file_name),
            file_name,
            size: std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0),
            version: String::new(),
            author: String::new(),
            description: String::new(),
            minecraft_version: String::new(),
            loader: None,
            loader_version: None,
            loader_label: LoaderType::Vanilla.label().to_string(),
            files: 0,
            settings: InstanceSettings::default(),
            blocked: None,
        };

        match &self.contents {
            Contents::Modrinth(index) => {
                if !index.name.trim().is_empty() {
                    pack.name = index.name.trim().to_string();
                }

                pack.version = index.version_id.clone();
                pack.description = index.summary.clone().unwrap_or_default();
                pack.files = index.files.iter().filter(|file| file.needed_on_client()).count();

                fill(&mut pack, index.minecraft_version().map(str::to_string), index.loader());
            }
            Contents::CurseForge(manifest) => {
                if !manifest.name.trim().is_empty() {
                    pack.name = manifest.name.trim().to_string();
                }

                pack.version = manifest.version.clone();
                pack.author = manifest.author.clone();
                pack.files = manifest.files.len();

                fill(
                    &mut pack,
                    manifest.minecraft_version().map(str::to_string),
                    manifest.loader(),
                );

                if !crate::curseforge::is_available() && pack.blocked.is_none() {
                    pack.blocked = Some("лаунчер собран без ключа CurseForge API".into());
                }
            }
            Contents::MultiMc { scanned, game_dir } => {
                pack.name = scanned.name.clone();
                pack.description = scanned.description.clone();
                pack.minecraft_version = scanned.minecraft_version.clone();
                pack.loader = scanned.loader;
                pack.loader_version = scanned.loader_version.clone();
                pack.loader_label = scanned.loader_label.clone();
                pack.settings = scanned.settings.clone();
                pack.blocked = scanned.blocked.clone();

                if game_dir.is_none() && pack.blocked.is_none() {
                    pack.blocked = Some("в архиве нет папки с файлами игры".into());
                }
            }
        }

        pack
    }

    async fn resolve(&self, minecraft_dir: &Path) -> CommandResult<ResolvedPack> {
        let mut resolved = match &self.contents {
            Contents::Modrinth(index) => index.resolve(minecraft_dir)?,
            Contents::CurseForge(manifest) => {
                crate::curseforge::pack::resolve(manifest, minecraft_dir).await?
            }
            Contents::MultiMc { scanned, game_dir } => {
                if let Some(reason) = &scanned.blocked {
                    return Err(CommandError::manifest(format!(
                        "Сборку «{}» перенести нельзя: {reason}",
                        scanned.name
                    )));
                }

                let game_dir = game_dir
                    .clone()
                    .ok_or_else(|| CommandError::manifest("В архиве нет папки с файлами игры"))?;

                let loader = scanned
                    .loader
                    .ok_or_else(|| CommandError::manifest("В архиве не указан загрузчик"))?;

                ResolvedPack {
                    minecraft_version: scanned.minecraft_version.clone(),
                    loader,
                    loader_version: scanned.loader_version.clone(),
                    tasks: Vec::new(),
                    paths: Vec::new(),
                    overrides: vec![game_dir],
                    blocked: Vec::new(),
                    recommended_ram: None,
                    seed: Vec::new(),
                    delete: Vec::new(),
                }
            }
        };

        resolved.overrides = resolved
            .overrides
            .iter()
            .map(|prefix| format!("{}{prefix}", self.root))
            .collect();

        Ok(resolved)
    }
}

fn fill(
    pack: &mut LocalPack,
    minecraft: CommandResult<String>,
    loader: CommandResult<(LoaderType, Option<String>)>,
) {
    match minecraft {
        Ok(version) => pack.minecraft_version = version,
        Err(error) => {
            pack.blocked = Some(error.message);
            return;
        }
    }

    match loader {
        Ok((loader, version)) => {
            pack.loader = Some(loader);
            pack.loader_label = match &version {
                Some(version) => format!("{} {version}", loader.label()),
                None => loader.label().to_string(),
            };
            pack.loader_version = version;
        }
        Err(error) => pack.blocked = Some(error.message),
    }
}

fn open_blocking(path: &Path) -> CommandResult<Opened> {
    let mut archive = crate::archive::open(path)?;
    let names: BTreeSet<String> = archive.file_names().map(str::to_string).collect();

    let Some((kind, root)) = detect(&names) else {
        return Err(CommandError::manifest(format!(
            "Непонятный файл: внутри нет ни {INDEX_ENTRY}, ни {MANIFEST_ENTRY}, ни {CONFIG_FILE} ({})",
            file_name(path)
        )));
    };

    let contents = match kind {
        LocalPackKind::Modrinth => {
            Contents::Modrinth(Box::new(PackIndex::parse(&read(&mut archive, &format!("{root}{INDEX_ENTRY}"))?)?))
        }
        LocalPackKind::CurseForge => {
            Contents::CurseForge(Box::new(Manifest::parse(&read(&mut archive, &format!("{root}{MANIFEST_ENTRY}"))?)?))
        }
        LocalPackKind::MultiMc => {
            let config = text(&mut archive, &format!("{root}{CONFIG_FILE}"))?;
            let pack = text(&mut archive, &format!("{root}{PACK_FILE}")).unwrap_or_default();
            let folder = folder(path, &root);

            Contents::MultiMc {
                scanned: Box::new(prism::parse(&folder, &config, &pack)),
                game_dir: game_dir(&names, &root),
            }
        }
    };

    Ok(Opened { root, contents })
}

fn detect(names: &BTreeSet<String>) -> Option<(LocalPackKind, String)> {
    let mut roots = vec![String::new()];
    roots.extend(top_level_dirs(names));

    for root in roots {
        for (entry, kind) in [
            (INDEX_ENTRY, LocalPackKind::Modrinth),
            (MANIFEST_ENTRY, LocalPackKind::CurseForge),
            (CONFIG_FILE, LocalPackKind::MultiMc),
        ] {
            if names.contains(&format!("{root}{entry}")) {
                return Some((kind, root));
            }
        }
    }

    None
}

fn top_level_dirs(names: &BTreeSet<String>) -> Vec<String> {
    let mut dirs: Vec<String> = names
        .iter()
        .filter_map(|name| name.split_once('/'))
        .map(|(dir, _)| format!("{dir}/"))
        .collect();

    dirs.dedup();
    dirs.truncate(MAX_ROOTS);
    dirs
}

fn game_dir(names: &BTreeSet<String>, root: &str) -> Option<String> {
    GAME_DIRS.into_iter().find_map(|dir| {
        let prefix = format!("{root}{dir}/");

        names
            .iter()
            .any(|name| name.starts_with(&prefix) && name.len() > prefix.len())
            .then(|| dir.to_string())
    })
}

fn read(archive: &mut ZipArchive<File>, entry: &str) -> CommandResult<Vec<u8>> {
    let mut file = archive
        .by_name(entry)
        .map_err(|e| CommandError::archive(format!("В архиве нет файла {entry}")).with_details(e.to_string()))?;

    if file.size() > MAX_MANIFEST {
        return Err(CommandError::archive(format!("Слишком большой {entry} внутри архива")));
    }

    let mut bytes = Vec::with_capacity(file.size() as usize);

    file.read_to_end(&mut bytes)
        .map_err(|e| CommandError::archive(format!("Не удалось прочитать {entry}")).with_details(e.to_string()))?;

    Ok(bytes)
}

fn text(archive: &mut ZipArchive<File>, entry: &str) -> CommandResult<String> {
    Ok(String::from_utf8_lossy(&read(archive, entry)?).to_string())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn stem(file_name: &str) -> String {
    match PathBuf::from(file_name).file_stem() {
        Some(stem) => stem.to_string_lossy().to_string(),
        None => file_name.to_string(),
    }
}

fn folder(path: &Path, root: &str) -> String {
    match root.trim_end_matches('/') {
        "" => stem(&file_name(path)),
        dir => dir.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-local-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();

        for (name, text) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }

        writer.finish().unwrap();
    }

    const MRPACK_INDEX: &str = r#"{
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "5.4.0",
        "name": "Fabulously Optimized",
        "summary": "Быстрый пак",
        "dependencies": {"minecraft": "1.20.1", "fabric-loader": "0.15.7"},
        "files": [
            {"path": "mods/jei.jar", "hashes": {"sha1": "aaa"}, "downloads": ["https://cdn.modrinth.com/jei.jar"]},
            {"path": "mods/server.jar", "env": {"client": "unsupported", "server": "required"},
             "downloads": ["https://cdn.modrinth.com/server.jar"]}
        ]
    }"#;

    const CF_MANIFEST: &str = r#"{
        "manifestType": "minecraftModpack",
        "manifestVersion": 1,
        "name": "All the Mods 9",
        "version": "0.2.60",
        "author": "ATMTeam",
        "minecraft": {"version": "1.20.1", "modLoaders": [{"id": "forge-47.2.0", "primary": true}]},
        "files": [{"projectID": 1, "fileID": 2}],
        "overrides": "overrides"
    }"#;

    const MMC_PACK: &str = r#"{
        "components": [
            {"uid": "net.minecraft", "version": "1.20.1"},
            {"uid": "net.minecraftforge", "version": "47.4.13"}
        ]
    }"#;

    #[tokio::test]
    async fn an_mrpack_is_recognised_by_its_index() {
        let dir = temp_dir();
        let path = dir.join("fo.mrpack");

        write_zip(&path, &[
            (INDEX_ENTRY, MRPACK_INDEX),
            ("overrides/config/a.toml", "a"),
        ]);

        let pack = inspect(&path).await.unwrap();

        assert_eq!(pack.kind, LocalPackKind::Modrinth);
        assert_eq!(pack.name, "Fabulously Optimized");
        assert_eq!(pack.version, "5.4.0");
        assert_eq!(pack.description, "Быстрый пак");
        assert_eq!(pack.minecraft_version, "1.20.1");
        assert_eq!(pack.loader, Some(LoaderType::Fabric));
        assert_eq!(pack.loader_version.as_deref(), Some("0.15.7"));
        assert_eq!(pack.files, 1, "серверные файлы клиенту не нужны");
        assert_eq!(pack.file_name, "fo.mrpack");
        assert!(pack.size > 0);
        assert!(pack.is_importable());

        let resolved = resolve(&path, Path::new("/mc")).await.unwrap();

        assert_eq!(resolved.tasks.len(), 1);
        assert_eq!(resolved.loader, LoaderType::Fabric);
        assert_eq!(resolved.overrides, crate::modrinth::pack::OVERRIDES.iter().map(|p| p.to_string()).collect::<Vec<_>>());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_curseforge_archive_is_recognised_by_its_manifest() {
        let dir = temp_dir();
        let path = dir.join("atm9.zip");

        write_zip(&path, &[(MANIFEST_ENTRY, CF_MANIFEST), ("overrides/options.txt", "fov:80")]);

        let pack = inspect(&path).await.unwrap();

        assert_eq!(pack.kind, LocalPackKind::CurseForge);
        assert_eq!(pack.name, "All the Mods 9");
        assert_eq!(pack.version, "0.2.60");
        assert_eq!(pack.author, "ATMTeam");
        assert_eq!(pack.loader, Some(LoaderType::Forge));
        assert_eq!(pack.loader_version.as_deref(), Some("1.20.1-47.2.0"));
        assert_eq!(pack.files, 1);
        assert_eq!(
            pack.is_importable(),
            crate::curseforge::is_available(),
            "без ключа API ссылки на моды всё равно не найти"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_prism_export_is_recognised_by_its_instance_cfg() {
        let dir = temp_dir();
        let path = dir.join("TerraFirmaGreg.zip");

        write_zip(&path, &[
            ("TerraFirmaGreg/instance.cfg", "[General]\nname=TerraFirmaGreg\nnotes=Моё"),
            ("TerraFirmaGreg/mmc-pack.json", MMC_PACK),
            ("TerraFirmaGreg/.minecraft/options.txt", "fov:80"),
            ("TerraFirmaGreg/.minecraft/mods/jei.jar", "jar"),
        ]);

        let pack = inspect(&path).await.unwrap();

        assert_eq!(pack.kind, LocalPackKind::MultiMc);
        assert_eq!(pack.name, "TerraFirmaGreg");
        assert_eq!(pack.description, "Моё");
        assert_eq!(pack.minecraft_version, "1.20.1");
        assert_eq!(pack.loader, Some(LoaderType::Forge));
        assert_eq!(pack.loader_version.as_deref(), Some("1.20.1-47.4.13"));
        assert!(pack.is_importable());

        let resolved = resolve(&path, Path::new("/mc")).await.unwrap();

        assert!(resolved.tasks.is_empty(), "в экспорте MultiMC все файлы уже лежат внутри");
        assert_eq!(resolved.overrides, vec!["TerraFirmaGreg/.minecraft"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_pack_nested_in_a_folder_is_still_found() {
        let dir = temp_dir();
        let path = dir.join("nested.zip");

        write_zip(&path, &[
            ("Мой пак/modrinth.index.json", MRPACK_INDEX),
            ("Мой пак/overrides/options.txt", "fov:80"),
        ]);

        let pack = inspect(&path).await.unwrap();
        assert_eq!(pack.kind, LocalPackKind::Modrinth);

        let resolved = resolve(&path, Path::new("/mc")).await.unwrap();

        assert_eq!(resolved.overrides, vec!["Мой пак/overrides", "Мой пак/client-overrides"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn the_resolved_prefixes_are_the_ones_the_unpacker_understands() {
        let dir = temp_dir();
        let minecraft = dir.join("minecraft");

        let prism = dir.join("prism.zip");
        write_zip(&prism, &[
            ("Сборка/instance.cfg", "[General]\nname=Сборка"),
            ("Сборка/mmc-pack.json", MMC_PACK),
            ("Сборка/.minecraft/mods/jei.jar", "jar"),
            ("Сборка/.minecraft/config/jei/a.toml", "a"),
            ("Сборка/лишнее.txt", "не из игры"),
        ]);

        let resolved = resolve(&prism, &minecraft).await.unwrap();
        let mut extracted = Vec::new();

        for prefix in &resolved.overrides {
            extracted.extend(
                crate::archive::extract_dir(prism.clone(), prefix.clone(), minecraft.clone())
                    .await
                    .unwrap(),
            );
        }

        extracted.sort();

        assert_eq!(extracted, vec!["config/jei/a.toml", "mods/jei.jar"]);
        assert!(minecraft.join("mods").join("jei.jar").is_file());
        assert!(!minecraft.join("лишнее.txt").exists(), "берём только папку игры");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_unsupported_loader_is_reported_instead_of_importing_a_broken_instance() {
        let dir = temp_dir();
        let path = dir.join("quilt.mrpack");

        write_zip(&path, &[(
            INDEX_ENTRY,
            r#"{"formatVersion": 1, "game": "minecraft", "name": "Quilt",
                 "dependencies": {"minecraft": "1.20.1", "quilt-loader": "0.23.1"}}"#,
        )]);

        let pack = inspect(&path).await.unwrap();

        assert!(!pack.is_importable());
        assert!(pack.blocked.unwrap().contains("Quilt"));
        assert!(resolve(&path, Path::new("/mc")).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_multimc_export_without_a_game_folder_cannot_be_imported() {
        let dir = temp_dir();
        let path = dir.join("empty.zip");

        write_zip(&path, &[("instance.cfg", "[General]\nname=x"), (PACK_FILE, MMC_PACK)]);

        let pack = inspect(&path).await.unwrap();

        assert!(!pack.is_importable());
        assert!(resolve(&path, Path::new("/mc")).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_file_of_an_unknown_shape_is_refused_with_a_hint() {
        let dir = temp_dir();

        let stranger = dir.join("stranger.zip");
        write_zip(&stranger, &[("readme.txt", "hello")]);

        let error = inspect(&stranger).await.unwrap_err();
        assert!(error.message.contains(INDEX_ENTRY), "{}", error.message);

        let broken = dir.join("broken.mrpack");
        std::fs::write(&broken, b"not a zip at all").unwrap();
        assert!(inspect(&broken).await.is_err());

        assert!(inspect(&dir.join("нет.mrpack")).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_nameless_pack_falls_back_to_the_file_name() {
        let dir = temp_dir();
        let path = dir.join("Мой Модпак.mrpack");

        write_zip(&path, &[(
            INDEX_ENTRY,
            r#"{"formatVersion": 1, "game": "minecraft", "dependencies": {"minecraft": "1.20.1"}}"#,
        )]);

        let pack = inspect(&path).await.unwrap();

        assert_eq!(pack.name, "Мой Модпак");
        assert_eq!(pack.loader, Some(LoaderType::Vanilla));
        assert!(pack.is_importable());

        std::fs::remove_dir_all(&dir).ok();
    }
}
