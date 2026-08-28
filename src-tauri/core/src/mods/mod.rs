pub mod catalog;
pub mod hash;
pub mod icon;
pub mod index;
pub mod install;
pub mod manage;
pub mod parse;
pub mod updates;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::error::{CommandError, CommandResult};
use crate::instance::LoaderType;

use index::ModsIndex;

pub const FOLDER: &str = "mods";

pub const DISABLED_SUFFIX: &str = ".disabled";

const EXTENSIONS: &[&str] = &["jar", "zip", "litemod"];

const CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    LiteLoader,
}

impl ModLoader {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
            Self::LiteLoader => "LiteLoader",
        }
    }

    pub fn of(loader: LoaderType) -> Option<Self> {
        match loader {
            LoaderType::Vanilla => None,
            LoaderType::Fabric => Some(Self::Fabric),
            LoaderType::Forge => Some(Self::Forge),
            LoaderType::NeoForge => Some(Self::NeoForge),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModKind {
    Jar,
    Folder,
    Litemod,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModDetails {
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub homepage: String,
    pub license: String,
    pub loaders: Vec<ModLoader>,
    pub icon_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub path: String,
    pub file_name: String,
    pub enabled: bool,
    pub size: u64,
    pub modified: u64,
    pub kind: ModKind,
    pub managed: bool,
    pub details: ModDetails,
}

impl ModFile {
    pub fn display_name(&self) -> &str {
        match self.details.name.trim().is_empty() {
            true => &self.file_name,
            false => &self.details.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModsScan {
    pub dir: PathBuf,
    pub index_file: PathBuf,
    pub icons: PathBuf,
    pub loader: Option<ModLoader>,
    pub managed: BTreeSet<String>,
}

impl ModsScan {
    fn is_managed(&self, path: &str) -> bool {
        self.managed.contains(path)
            || self.managed.contains(path.trim_end_matches(DISABLED_SUFFIX))
    }
}

#[derive(Debug, Clone)]
struct Entry {
    path: String,
    file_name: String,
    enabled: bool,
    size: u64,
    modified: u64,
    kind: ModKind,
}

pub async fn list(scan: &ModsScan, force: bool) -> CommandResult<Vec<ModFile>> {
    let entries = collect(&scan.dir).await?;

    let mut index = match force {
        true => ModsIndex::new(),
        false => ModsIndex::load(&scan.index_file).await,
    };

    let mut pending = Vec::new();
    let mut parsed = Vec::new();
    let mut mods = Vec::with_capacity(entries.len());

    for entry in &entries {
        match index
            .reusable(&entry.path, entry.size, entry.modified)
            .filter(|details| icon_present(&scan.icons, details))
        {
            Some(details) => mods.push(file(scan, entry, details.clone())),
            None => pending.push(entry.clone()),
        }
    }

    if !pending.is_empty() {
        crate::fs_util::ensure_dir(&scan.icons).await?;

        parsed = parse_all(pending, scan).await;

        for (entry, details) in &parsed {
            index.remember(&entry.path, entry.size, entry.modified, details);
            mods.push(file(scan, entry, details.clone()));
        }
    }

    let forgotten = index.retain(&entries.iter().map(|entry| entry.path.clone()).collect::<Vec<_>>());

    if !parsed.is_empty() || forgotten {
        if let Err(error) = index.save(&scan.index_file).await {
            eprintln!("Не удалось сохранить кэш модов: {}", error.message);
        }
    }

    mods.sort_by(|a, b| {
        a.display_name()
            .to_lowercase()
            .cmp(&b.display_name().to_lowercase())
            .then_with(|| a.file_name.cmp(&b.file_name))
    });

    Ok(mods)
}

fn file(scan: &ModsScan, entry: &Entry, details: ModDetails) -> ModFile {
    ModFile {
        managed: scan.is_managed(&entry.path),
        path: entry.path.clone(),
        file_name: entry.file_name.clone(),
        enabled: entry.enabled,
        size: entry.size,
        modified: entry.modified,
        kind: entry.kind,
        details,
    }
}

fn icon_present(icons: &Path, details: &ModDetails) -> bool {
    match &details.icon_key {
        Some(key) => icon::exists(icons, key),
        None => true,
    }
}

async fn collect(dir: &Path) -> CommandResult<Vec<Entry>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог модов", dir, e))?;

    let mut found = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог модов", dir, e))?
    {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Ok(metadata) = entry.metadata().await else { continue };

        let Some(kind) = kind_of(&file_name, metadata.is_dir()) else { continue };

        if kind == ModKind::Folder && !is_mod_folder(&entry.path()) {
            continue;
        }

        found.push(Entry {
            path: format!("{FOLDER}/{file_name}"),
            enabled: !file_name.ends_with(DISABLED_SUFFIX),
            size: match metadata.is_dir() {
                true => 0,
                false => metadata.len(),
            },
            modified: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|since| since.as_millis() as u64)
                .unwrap_or(0),
            kind,
            file_name,
        });
    }

    Ok(found)
}

pub fn is_mod_file(file_name: &str) -> bool {
    kind_of(file_name, false).is_some()
}

fn kind_of(file_name: &str, is_dir: bool) -> Option<ModKind> {
    let name = file_name.trim_end_matches(DISABLED_SUFFIX);

    if is_dir {
        return Some(ModKind::Folder);
    }

    let extension = Path::new(name).extension()?.to_string_lossy().to_lowercase();

    if !EXTENSIONS.contains(&extension.as_str()) {
        return None;
    }

    Some(match extension.as_str() {
        "litemod" => ModKind::Litemod,
        _ => ModKind::Jar,
    })
}

fn is_mod_folder(path: &Path) -> bool {
    const MARKERS: &[&str] = &[
        "fabric.mod.json",
        "quilt.mod.json",
        "mcmod.info",
        "META-INF/mods.toml",
        "META-INF/neoforge.mods.toml",
    ];

    MARKERS.iter().any(|marker| {
        marker
            .split('/')
            .fold(path.to_path_buf(), |acc, part| acc.join(part))
            .is_file()
    })
}

async fn parse_all(pending: Vec<Entry>, scan: &ModsScan) -> Vec<(Entry, ModDetails)> {
    let mut queue = pending.into_iter();
    let mut tasks = JoinSet::new();
    let mut parsed = Vec::new();

    for _ in 0..CONCURRENCY {
        match queue.next() {
            Some(entry) => spawn(&mut tasks, entry, scan),
            None => break,
        }
    }

    while let Some(joined) = tasks.join_next().await {
        if let Ok(result) = joined {
            parsed.push(result);
        }

        if let Some(entry) = queue.next() {
            spawn(&mut tasks, entry, scan);
        }
    }

    parsed
}

fn spawn(tasks: &mut JoinSet<(Entry, ModDetails)>, entry: Entry, scan: &ModsScan) {
    let path = scan.dir.join(&entry.file_name);
    let icons = scan.icons.clone();
    let loader = scan.loader;

    tasks.spawn_blocking(move || {
        let details = details(&path, &entry, &icons, loader);
        (entry, details)
    });
}

fn details(path: &Path, entry: &Entry, icons: &Path, loader: Option<ModLoader>) -> ModDetails {
    let parsed = match entry.kind {
        ModKind::Folder => parse::folder(path, loader),
        _ => parse::jar(path, loader),
    };

    let parsed = parsed.unwrap_or_else(|_| parse::Parsed {
        details: ModDetails::default(),
        icon: None,
    });

    let mut details = parsed.details;

    if details.name.trim().is_empty() {
        let (name, version) = parse::from_file_name(&entry.file_name);

        details.name = name;

        if details.version.trim().is_empty() {
            details.version = version;
        }
    }

    if let Some(raw) = parsed.icon {
        let key = icon::key(&entry.path, entry.size, entry.modified, raw.extension);

        if icon::store(icons, &key, &raw.bytes).is_ok() {
            details.icon_key = Some(key);
        }
    }

    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-mods-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn scan_in(root: &Path) -> ModsScan {
        ModsScan {
            dir: root.join("minecraft").join(FOLDER),
            index_file: root.join("mods-index.json"),
            icons: root.join("mod-icons"),
            loader: Some(ModLoader::Fabric),
            managed: BTreeSet::from([
                "mods/sodium.jar".to_string(),
                "mods/sodium-0.5.3.jar".to_string(),
            ]),
        }
    }

    fn png() -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(b"pixels");
        bytes
    }

    fn write_jar(path: &Path, files: &[(&str, Vec<u8>)]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, bytes) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }

        zip.finish().unwrap();
    }

    fn fabric_jar(path: &Path, id: &str, name: &str, with_icon: bool) {
        let mut files = vec![(
            "fabric.mod.json",
            serde_json::to_vec(&serde_json::json!({
                "id": id,
                "name": name,
                "version": "1.0.0",
                "icon": "icon.png"
            }))
            .unwrap(),
        )];

        if with_icon {
            files.push(("icon.png", png()));
        }

        write_jar(path, &files);
    }

    #[tokio::test]
    async fn mods_are_listed_with_names_icons_and_switches() {
        let root = temp_dir();
        let scan = scan_in(&root);

        fabric_jar(&scan.dir.join("sodium-0.5.3.jar"), "sodium", "Sodium", true);
        fabric_jar(&scan.dir.join("lithium.jar.disabled"), "lithium", "Lithium", false);
        std::fs::write(scan.dir.join("notes.txt"), "не мод".as_bytes()).unwrap();

        let mods = list(&scan, false).await.unwrap();

        assert_eq!(mods.len(), 2, "текстовый файл в список не попал");

        let lithium = &mods[0];
        assert_eq!(lithium.display_name(), "Lithium");
        assert!(!lithium.enabled, "суффикс .disabled - мод выключен");
        assert_eq!(lithium.details.icon_key, None);

        assert!(!lithium.managed);

        let sodium = &mods[1];
        assert_eq!(sodium.display_name(), "Sodium");
        assert!(sodium.managed, "мод из пака помечен");
        assert_eq!(sodium.path, "mods/sodium-0.5.3.jar");
        assert!(sodium.enabled);
        assert_eq!(sodium.details.version, "1.0.0");

        let key = sodium.details.icon_key.clone().expect("иконка вынута из jar");
        assert!(icon::exists(&scan.icons, &key));
        assert!(icon::data_url(&scan.icons, &key).await.unwrap().starts_with("data:image/png;base64,"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_jar_without_metadata_falls_back_to_its_file_name() {
        let root = temp_dir();
        let scan = scan_in(&root);

        write_jar(&scan.dir.join("mysterymod-2.4.jar"), &[("data.bin", b"x".to_vec())]);
        std::fs::write(scan.dir.join("broken-1.0.jar"), "это вообще не архив".as_bytes()).unwrap();

        let mods = list(&scan, false).await.unwrap();

        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].display_name(), "broken");
        assert_eq!(mods[0].details.version, "1.0");
        assert_eq!(mods[1].display_name(), "mysterymod");
        assert_eq!(mods[1].details.version, "2.4");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_second_scan_reuses_the_cache_and_forgets_deleted_mods() {
        let root = temp_dir();
        let scan = scan_in(&root);

        fabric_jar(&scan.dir.join("sodium.jar"), "sodium", "Sodium", true);
        fabric_jar(&scan.dir.join("lithium.jar"), "lithium", "Lithium", false);

        assert_eq!(list(&scan, false).await.unwrap().len(), 2);

        std::fs::write(scan.dir.join("sodium.jar"), "мусор поверх архива".as_bytes()).unwrap();
        let sizes_changed = list(&scan, false).await.unwrap();
        assert_eq!(sizes_changed[1].display_name(), "sodium", "размер изменился - разбираем заново");

        std::fs::remove_file(scan.dir.join("lithium.jar")).unwrap();
        let mods = list(&scan, false).await.unwrap();

        assert_eq!(mods.len(), 1);

        let index = ModsIndex::load(&scan.index_file).await;
        assert!(!index.entries.contains_key("mods/lithium.jar"), "удалённый мод ушёл из кэша");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn an_unchanged_folder_does_not_rewrite_the_cache() {
        let root = temp_dir();
        let scan = scan_in(&root);

        fabric_jar(&scan.dir.join("sodium.jar"), "sodium", "Sodium", true);

        list(&scan, false).await.unwrap();
        let written = std::fs::metadata(&scan.index_file).unwrap().modified().unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        list(&scan, false).await.unwrap();

        assert_eq!(
            std::fs::metadata(&scan.index_file).unwrap().modified().unwrap(),
            written,
            "ничего не изменилось - кэш не переписан"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_missing_mods_folder_is_an_empty_list_not_an_error() {
        let root = temp_dir();
        let mods = list(&scan_in(&root), false).await.unwrap();

        assert!(mods.is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn only_folders_with_metadata_count_as_mods() {
        let root = temp_dir();
        let scan = scan_in(&root);

        let unpacked = scan.dir.join("unpacked");
        std::fs::create_dir_all(&unpacked).unwrap();
        std::fs::write(
            unpacked.join("fabric.mod.json"),
            serde_json::to_vec(&serde_json::json!({"id": "unpacked", "name": "Unpacked", "version": "3.0"})).unwrap(),
        )
        .unwrap();

        let junk = scan.dir.join("memory_repo");
        std::fs::create_dir_all(&junk).unwrap();
        std::fs::write(junk.join("cache.bin"), b"x").unwrap();

        let mods = list(&scan, false).await.unwrap();

        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].display_name(), "Unpacked");
        assert_eq!(mods[0].kind, ModKind::Folder);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_switched_off_mod_from_a_pack_is_still_a_mod_from_a_pack() {
        let root = temp_dir();
        let scan = scan_in(&root);

        fabric_jar(&scan.dir.join("sodium.jar.disabled"), "sodium", "Sodium", false);

        let mods = list(&scan, false).await.unwrap();

        assert!(mods[0].managed);
        assert!(!mods[0].enabled);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn only_mod_shaped_files_are_picked_up() {
        assert_eq!(kind_of("jei.jar", false), Some(ModKind::Jar));
        assert_eq!(kind_of("jei.JAR", false), Some(ModKind::Jar));
        assert_eq!(kind_of("jei.jar.disabled", false), Some(ModKind::Jar));
        assert_eq!(kind_of("old.litemod", false), Some(ModKind::Litemod));
        assert_eq!(kind_of("pack.zip", false), Some(ModKind::Jar));
        assert_eq!(kind_of("readme.txt", false), None);
        assert_eq!(kind_of("jei.jar.bak", false), None);
        assert_eq!(kind_of("anything", true), Some(ModKind::Folder));
    }

    #[test]
    fn the_instance_loader_picks_the_metadata_to_trust() {
        assert_eq!(ModLoader::of(LoaderType::Fabric), Some(ModLoader::Fabric));
        assert_eq!(ModLoader::of(LoaderType::NeoForge), Some(ModLoader::NeoForge));
        assert_eq!(ModLoader::of(LoaderType::Vanilla), None);
    }
}
