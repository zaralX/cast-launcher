use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{relative_key, safe_join};
use crate::instance::{Instance, LoaderType, PackProvider};
use crate::mods::catalog::{self, CatalogMatch};
use crate::mods::hash::{self, FileHashes};
use crate::mods::{self, ModsScan, DISABLED_SUFFIX};

use super::file::{forbidden, BaseInfo, CastFile, FORMAT_VERSION};
use super::manifest::{
    BaseSpec, FileMode, LoaderSpec, Manifest, ModEntry, PackSettings, MAX_FILE_ENTRIES,
};
use super::mods::ResolvedMods;
use super::SCHEMA_VERSION;

/// Folders whose files may come from Modrinth or CurseForge and then travel as links.
pub const CATALOG_DIRS: [&str; 3] = ["mods", "resourcepacks", "shaderpacks"];

const HIDDEN: &[&str] = &["natives", "client.jar"];

const JUNK: &[&str] = &["thumbs.db", "desktop.ini", ".ds_store"];

const WORLD: &[&str] = &["saves"];

const PERSONAL: &[&str] = &[
    "screenshots",
    "logs",
    "crash-reports",
    "servers.dat",
    "servers.dat_old",
    "usercache.json",
    "usernamecache.json",
    "command_history.txt",
    "essential",
    "realms_persistence.json",
    "patchouli_data.json",
    "schematics",
    "replay_recordings",
    "replay_videos",
    "backups",
    "local",
];

const CACHE: &[&str] = &[
    ".cache",
    ".fabric",
    ".mixin.out",
    ".connector",
    "debug",
    "downloads",
    "journeymap",
    "xaero",
    "xaeroworldmap",
    "xaerowaypoints",
    "distant_horizons_server_data",
    "launcher_profiles.json",
];

/// Settings a player tunes for themselves: the pack only brings them when there are none yet.
const ONCE: &[&str] = &[
    "options.txt",
    "optionsof.txt",
    "optionsshaders.txt",
    "servers.dat",
];

const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Note {
    World,
    Personal,
    Cache,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub hidden: bool,
    pub selected: bool,
    pub mode: FileMode,
    pub note: Option<Note>,
}

/// What an export does with a path of the game folder unless the player says otherwise.
pub fn choice(key: &str) -> Choice {
    let name = key.rsplit('/').next().unwrap_or(key).to_ascii_lowercase();
    let top = key.split('/').next().unwrap_or(key).to_ascii_lowercase();
    let nested = key.contains('/');

    let mut choice = Choice {
        hidden: false,
        selected: true,
        mode: FileMode::Always,
        note: None,
    };

    if JUNK.contains(&name.as_str()) || (name.starts_with('.') && name.ends_with(".tmp")) {
        choice.hidden = true;
        return choice;
    }

    if HIDDEN.contains(&top.as_str()) {
        choice.hidden = true;
        return choice;
    }

    if forbidden(key) {
        choice.selected = false;
        choice.note = Some(Note::Forbidden);
        return choice;
    }

    if !nested && ONCE.contains(&name.as_str()) {
        choice.mode = FileMode::Once;
    }

    let note = if WORLD.contains(&top.as_str()) {
        Some(Note::World)
    } else if PERSONAL.contains(&top.as_str()) {
        Some(Note::Personal)
    } else if CACHE.contains(&top.as_str()) || (!nested && name.ends_with(".log")) {
        Some(Note::Cache)
    } else {
        None
    };

    if note.is_some() {
        choice.selected = false;
        choice.note = note;
    }

    choice
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Modrinth,
    CurseForge,
    Embedded,
    Unchecked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSource {
    pub kind: SourceKind,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub key: String,
    pub name: String,
    pub dir: bool,
    pub size: u64,
    pub files: u64,
    pub selected: bool,
    pub mode: FileMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Note>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FileSource>,
    /// The base modpack brings the same file.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub in_base: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeEntry>,
}

/// The game folder two levels deep, with sizes counted all the way down.
pub fn scan_tree(minecraft_dir: &Path) -> CommandResult<Vec<TreeEntry>> {
    if !minecraft_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    for (name, path, is_dir) in list_dir(minecraft_dir)? {
        let key = name.clone();
        let choice = choice(&key);

        if choice.hidden {
            continue;
        }

        let mut entry = leaf(&key, &name, &path, is_dir, choice);

        if is_dir {
            for (child_name, child_path, child_dir) in list_dir(&path)? {
                let child_key = format!("{key}/{child_name}");
                let mut child_choice = self::choice(&child_key);

                if child_choice.hidden {
                    continue;
                }

                if child_choice.note.is_none() {
                    child_choice.selected = choice.selected;
                }

                let child = leaf(
                    &child_key,
                    &child_name,
                    &child_path,
                    child_dir,
                    child_choice,
                );

                entry.size += child.size;
                entry.files += child.files;
                entry.children.push(child);
            }
        }

        entries.push(entry);
    }

    Ok(entries)
}

fn leaf(key: &str, name: &str, path: &Path, is_dir: bool, choice: Choice) -> TreeEntry {
    let (size, files) = match is_dir {
        true => match key.contains('/') {
            true => measure(path, 0),
            false => (0, 0),
        },
        false => (
            std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0),
            1,
        ),
    };

    TreeEntry {
        key: key.to_string(),
        name: name.to_string(),
        dir: is_dir,
        size,
        files,
        selected: choice.selected,
        mode: choice.mode,
        note: choice.note,
        source: None,
        in_base: false,
        children: Vec::new(),
    }
}

fn measure(dir: &Path, depth: usize) -> (u64, u64) {
    if depth > MAX_DEPTH {
        return (0, 0);
    }

    let Ok(entries) = list_dir(dir) else {
        return (0, 0);
    };

    let mut size = 0;
    let mut files = 0;

    for (_, path, is_dir) in entries {
        if is_dir {
            let (inner_size, inner_files) = measure(&path, depth + 1);
            size += inner_size;
            files += inner_files;
        } else {
            size += std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            files += 1;
        }
    }

    (size, files)
}

/// Entries of a folder sorted folders first, without symlinks: a link could lead out of the game folder.
fn list_dir(dir: &Path) -> CommandResult<Vec<(String, PathBuf, bool)>> {
    let mut found = Vec::new();

    for entry in std::fs::read_dir(dir)
        .map_err(|e| CommandError::io("error.reason.fs.read_dir", dir, e))?
        .flatten()
    {
        let Ok(kind) = entry.file_type() else {
            continue;
        };

        if kind.is_symlink() || !(kind.is_dir() || kind.is_file()) {
            continue;
        }

        found.push((
            entry.file_name().to_string_lossy().to_string(),
            entry.path(),
            kind.is_dir(),
        ));
    }

    found.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
    });

    Ok(found)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFile {
    pub key: String,
    pub path: PathBuf,
    pub size: u64,
    pub mode: FileMode,
}

#[derive(Debug, Default)]
pub struct Collected {
    pub files: Vec<ExportFile>,
    /// Programs inside the chosen folders that stay out of the pack.
    pub skipped: Vec<String>,
}

/// Every file under the chosen paths, without hidden, junk or forbidden ones.
pub fn collect(minecraft_dir: &Path, include: &[String]) -> CommandResult<Collected> {
    let mut files: BTreeMap<String, ExportFile> = BTreeMap::new();
    let mut skipped = BTreeSet::new();

    for chosen in include {
        let key = relative_key(chosen)?;
        let path = safe_join(minecraft_dir, &key)?;

        if choice(&key).hidden {
            continue;
        }

        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            log::warn!("Export skips '{key}': it is gone from the game folder");
            continue;
        };

        if meta.is_dir() {
            walk(&path, &key, 0, &mut files, &mut skipped)?;
        } else if meta.is_file() {
            add(&key, path, meta.len(), &mut files, &mut skipped);
        }
    }

    Ok(Collected {
        files: files.into_values().collect(),
        skipped: skipped.into_iter().collect(),
    })
}

fn walk(
    dir: &Path,
    key: &str,
    depth: usize,
    files: &mut BTreeMap<String, ExportFile>,
    skipped: &mut BTreeSet<String>,
) -> CommandResult<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }

    for (name, path, is_dir) in list_dir(dir)? {
        let child = format!("{key}/{name}");

        if choice(&child).hidden {
            continue;
        }

        if is_dir {
            walk(&path, &child, depth + 1, files, skipped)?;
        } else {
            let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
            add(&child, path, size, files, skipped);
        }
    }

    Ok(())
}

fn add(
    key: &str,
    path: PathBuf,
    size: u64,
    files: &mut BTreeMap<String, ExportFile>,
    skipped: &mut BTreeSet<String>,
) {
    let choice = choice(key);

    if choice.note == Some(Note::Forbidden) {
        skipped.insert(key.to_string());
        return;
    }

    // A name Windows would rewrite cannot travel in a pack at all.
    if relative_key(key).is_err() {
        skipped.insert(key.to_string());
        return;
    }

    files.insert(
        key.to_string(),
        ExportFile {
            key: key.to_string(),
            path,
            size,
            mode: choice.mode,
        },
    );
}

/// A direct child of a catalog folder: only those can stand for a catalog file.
pub fn is_catalog_candidate(key: &str) -> bool {
    match key.split_once('/') {
        Some((dir, name)) => CATALOG_DIRS.contains(&dir) && !name.contains('/'),
        None => false,
    }
}

#[derive(Debug, Default)]
pub struct Lookup {
    /// Files the importer gets from the catalogs exactly as they are here.
    pub found: BTreeMap<String, CatalogMatch>,
    /// Where the importer puts each found file, which may differ from its name here.
    pub targets: BTreeMap<String, String>,
    pub unchecked: BTreeSet<String>,
}

impl Lookup {
    pub fn source(&self, key: &str) -> FileSource {
        match self.found.get(key) {
            Some(found) => FileSource {
                kind: match found.provider {
                    PackProvider::Modrinth => SourceKind::Modrinth,
                    PackProvider::CurseForge => SourceKind::CurseForge,
                },
                title: found.title.clone(),
            },
            None if self.unchecked.contains(key) => FileSource {
                kind: SourceKind::Unchecked,
                title: String::new(),
            },
            None => FileSource {
                kind: SourceKind::Embedded,
                title: String::new(),
            },
        }
    }
}

/// Asks the catalogs about the files, then resolves the answers the way an import would and
/// keeps only those that come back as the same file in the same folder.
pub async fn lookup(
    files: &[(String, PathBuf)],
    scan: &ModsScan,
    minecraft_dir: &Path,
    cache_file: &Path,
) -> CommandResult<Lookup> {
    let wanted: Vec<(String, PathBuf)> = files
        .iter()
        .filter(|(key, _)| is_catalog_candidate(key))
        .cloned()
        .collect();

    if wanted.is_empty() {
        return Ok(Lookup::default());
    }

    let hashes = hash_files(&wanted, scan).await?;
    let identified = catalog::identify_hashes(&hashes, cache_file).await;

    let entries: Vec<ModEntry> = identified
        .found
        .iter()
        .map(|(key, found)| entry(key, found))
        .collect();

    let refs = entries
        .iter()
        .map(ModEntry::reference)
        .collect::<CommandResult<Vec<_>>>()?;

    let (found, targets) = match super::mods::resolve(&refs, minecraft_dir).await {
        Ok(resolved) => verified(&identified.found, &hashes, &resolved),
        Err(error) => {
            log::warn!("Could not check the identified files against the catalogs, trusting the hashes: {error}");

            let targets = identified
                .found
                .keys()
                .map(|key| (key.clone(), key.clone()))
                .collect();

            (identified.found, targets)
        }
    };

    Ok(Lookup {
        found,
        targets,
        unchecked: identified.unchecked,
    })
}

/// Hashes of the files, reusing what the mod index knows about the mods folder.
pub async fn hash_files(
    files: &[(String, PathBuf)],
    scan: &ModsScan,
) -> CommandResult<BTreeMap<String, FileHashes>> {
    let mods_prefix = format!("{}/", mods::FOLDER);

    let mut hashes = match files.iter().any(|(key, _)| key.starts_with(&mods_prefix)) {
        true => {
            let mods_list = mods::list(scan, false).await?;
            catalog::mod_hashes(scan, &mods_list).await?
        }
        false => BTreeMap::new(),
    };

    hashes.retain(|key, _| files.iter().any(|(wanted, _)| wanted == key));

    let pending: Vec<(String, PathBuf)> = files
        .iter()
        .filter(|(key, _)| !(key.starts_with(&mods_prefix) && hashes.contains_key(key)))
        .map(|(key, path)| (key.clone(), path.clone()))
        .collect();
    let mut pending = pending.into_iter();

    let mut tasks = JoinSet::new();

    for _ in 0..8 {
        if let Some((key, path)) = pending.next() {
            tasks.spawn(async move { (key, hash::of(&path).await) });
        }
    }

    while let Some(joined) = tasks.join_next().await {
        if let Ok((key, Some(found))) = joined {
            hashes.insert(key, found);
        }

        if let Some((key, path)) = pending.next() {
            tasks.spawn(async move { (key, hash::of(&path).await) });
        }
    }

    Ok(hashes)
}

fn entry(key: &str, found: &CatalogMatch) -> ModEntry {
    ModEntry {
        provider: Some(found.provider),
        project_id: found.project_id.clone(),
        version_id: found.version_id.clone(),
        optional: key.ends_with(DISABLED_SUFFIX),
        ..Default::default()
    }
}

fn parent(key: &str) -> &str {
    key.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
}

fn plain_name(key: &str) -> &str {
    let name = key.rsplit('/').next().unwrap_or(key);
    name.strip_suffix(DISABLED_SUFFIX).unwrap_or(name)
}

/// The same hash in the same folder, or the same name when the catalog gives no hash.
fn same_file(local: &str, sha1: &str, resolved: &str, resolved_sha1: Option<&str>) -> bool {
    if parent(local) != parent(resolved) {
        return false;
    }

    match resolved_sha1 {
        Some(resolved_sha1) => resolved_sha1.eq_ignore_ascii_case(sha1),
        None => plain_name(local).eq_ignore_ascii_case(plain_name(resolved)),
    }
}

/// The found files the catalogs give back unchanged, with the key each one lands under.
pub fn verified(
    found: &BTreeMap<String, CatalogMatch>,
    hashes: &BTreeMap<String, FileHashes>,
    resolved: &ResolvedMods,
) -> (BTreeMap<String, CatalogMatch>, BTreeMap<String, String>) {
    let mut kept = BTreeMap::new();
    let mut targets = BTreeMap::new();

    for (key, matched) in found {
        let Some(sha1) = hashes.get(key).map(|hashes| hashes.sha1.as_str()) else {
            continue;
        };

        let target = resolved
            .files
            .iter()
            .find(|(other, task)| same_file(key, sha1, other, task.sha1.as_deref()))
            .map(|(other, _)| other.clone())
            .or_else(|| {
                resolved
                    .blocked
                    .iter()
                    .find(|file| same_file(key, sha1, &file.target_path, file.sha1.as_deref()))
                    .map(|file| file.target_path.clone())
            });

        match target {
            Some(target) => {
                kept.insert(key.clone(), matched.clone());
                targets.insert(key.clone(), target);
            }
            None => log::info!("Export embeds '{key}': the catalog gives a different file for it"),
        }
    }

    (kept, targets)
}

/// What a base modpack already brings: its downloads, manual downloads and overrides,
/// each with the hash it has there when the pack tells it.
pub type BaseFiles = BTreeMap<String, Option<String>>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Diff {
    /// Files here that the base brings as they are: the pack leaves them out.
    pub skip: BTreeSet<String>,
    /// Files of the base the pack takes away.
    pub delete: BTreeSet<String>,
    /// Files of the base the pack brings its own copy of.
    pub replaced: BTreeSet<String>,
}

/// Compares the game folder with the base pack. Only folders the export covers count: an
/// untouched folder stays as the base has it.
pub fn diff(
    base: &BaseFiles,
    local: &BTreeMap<String, String>,
    covered: impl Fn(&str) -> bool,
) -> Diff {
    let mut diff = Diff::default();

    for (key, base_sha1) in base {
        if !covered(key) {
            continue;
        }

        match local.get(key) {
            Some(sha1)
                if base_sha1
                    .as_deref()
                    .is_none_or(|base_sha1| base_sha1.eq_ignore_ascii_case(sha1)) =>
            {
                diff.skip.insert(key.clone());
            }
            Some(_) => {
                diff.replaced.insert(key.clone());
            }
            // Missing, or switched on or off by the player: the other copy travels on its own.
            None => {
                diff.delete.insert(key.clone());
            }
        }
    }

    diff
}

/// Whether the export speaks for the top folder of `key`: a folder the player left out
/// entirely is not taken away from the base.
pub fn covered_by(include: &[String]) -> impl Fn(&str) -> bool + '_ {
    move |key: &str| {
        let top = key.split('/').next().unwrap_or(key);

        include
            .iter()
            .any(|chosen| chosen.split('/').next().unwrap_or(chosen) == top)
    }
}

/// A file the pack replaces turns into a link that lands under another name: the copy of
/// the base must go, or both would stay.
pub fn stale_replacements(diff: &Diff, lookup: &Lookup) -> Vec<String> {
    diff.replaced
        .iter()
        .filter(|key| {
            lookup
                .targets
                .get(*key)
                .is_some_and(|target| target != *key)
        })
        .cloned()
        .collect()
}

/// Marks the files of the tree the base pack already brings.
pub fn mark_base(tree: &mut [TreeEntry], skip: &BTreeSet<String>) {
    for entry in tree.iter_mut() {
        entry.in_base = skip.contains(&entry.key);

        for child in entry.children.iter_mut() {
            child.in_base = skip.contains(&child.key);
        }
    }
}

/// Files of the tree that the base pack may bring: the ones to hash for a comparison.
pub fn base_candidates(
    tree: &[TreeEntry],
    base: &BaseFiles,
    minecraft_dir: &Path,
) -> Vec<(String, PathBuf)> {
    tree.iter()
        .flat_map(|entry| std::iter::once(entry).chain(entry.children.iter()))
        .filter(|entry| !entry.dir && base.contains_key(&entry.key))
        .filter_map(|entry| {
            Some((
                entry.key.clone(),
                safe_join(minecraft_dir, &entry.key).ok()?,
            ))
        })
        .collect()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExportRequest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub changelog: String,
    pub recommended_ram: Option<u32>,
    pub include: Vec<String>,
    /// Ship only the difference with the modpack the instance is built on.
    pub use_base: bool,
}

impl ExportRequest {
    pub fn normalized(self) -> CommandResult<Self> {
        let name = self.name.trim().to_string();
        let version = self.version.trim().to_string();

        if name.is_empty() {
            return Err(CommandError::invalid_input(
                "error.reason.cast.name_required",
            ));
        }

        if version.is_empty() {
            return Err(CommandError::invalid_input(
                "error.reason.cast.version_required",
            ));
        }

        let mut include = Vec::new();

        for chosen in &self.include {
            let key = relative_key(chosen)?;

            if !include.contains(&key) {
                include.push(key);
            }
        }

        if include.is_empty() {
            return Err(CommandError::invalid_input(
                "error.reason.cast.nothing_selected",
            ));
        }

        Ok(Self {
            name,
            version,
            author: self.author.trim().to_string(),
            description: self.description.trim().to_string(),
            changelog: self.changelog.trim().to_string(),
            recommended_ram: self.recommended_ram.filter(|ram| *ram > 0),
            include,
            use_base: self.use_base,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDefaults {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_ram: Option<u32>,
    /// Whether the pack goes out under the id of an earlier export or import.
    pub known_pack: bool,
}

/// The id the pack goes out under: the one it already has, so its players can update.
pub fn pack_id(instance: &Instance) -> Option<String> {
    instance
        .cast_export
        .as_ref()
        .map(|export| export.id.clone())
        .or_else(|| {
            instance
                .castpack
                .as_ref()
                .filter(|source| source.is_file())
                .map(|source| source.catalog_id.clone())
        })
        .filter(|id| !id.trim().is_empty())
}

pub fn defaults(instance: &Instance) -> ExportDefaults {
    let previous = instance.cast_export.as_ref();

    let version = previous
        .map(|export| export.version.clone())
        .or_else(|| {
            instance
                .castpack
                .as_ref()
                .filter(|source| source.is_file())
                .map(|source| source.version.clone())
        })
        .filter(|version| !version.trim().is_empty())
        .unwrap_or_else(|| "1.0.0".to_string());

    ExportDefaults {
        name: instance.name.clone(),
        version,
        author: previous
            .map(|export| export.author.clone())
            .unwrap_or_default(),
        description: instance.description.clone(),
        recommended_ram: (instance.settings.override_memory && instance.settings.max_ram > 0)
            .then_some(instance.settings.max_ram),
        known_pack: pack_id(instance).is_some(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportScan {
    pub tree: Vec<TreeEntry>,
    pub defaults: ExportDefaults,
    /// Files the catalogs could not be asked about: they travel inside the file.
    pub unchecked: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<ExportBase>,
}

/// The modpack the instance is built on, which an export may lean on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBase {
    pub provider: PackProvider,
    pub name: String,
    pub version: String,
    /// Whether the tree already marks what the base brings: it does once its archive is downloaded.
    pub compared: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportStage {
    Collecting,
    Identifying,
    Packing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub instance_id: String,
    pub stage: ExportStage,
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub size: u64,
    pub mods: usize,
    pub embedded: usize,
    pub embedded_code: usize,
    /// Files of the base modpack the pack takes away.
    pub removed: usize,
    pub skipped: Vec<String>,
}

/// Attaches where each file of a catalog folder would come from.
pub fn mark_sources(tree: &mut [TreeEntry], lookup: &Lookup) {
    for entry in tree.iter_mut() {
        for child in entry.children.iter_mut() {
            if !child.dir && is_catalog_candidate(&child.key) {
                child.source = Some(lookup.source(&child.key));
            }
        }
    }
}

/// Files of the tree that may be catalog files.
pub fn candidates(tree: &[TreeEntry], minecraft_dir: &Path) -> Vec<(String, PathBuf)> {
    tree.iter()
        .flat_map(|entry| entry.children.iter())
        .filter(|child| !child.dir && is_catalog_candidate(&child.key))
        .filter_map(|child| {
            Some((
                child.key.clone(),
                safe_join(minecraft_dir, &child.key).ok()?,
            ))
        })
        .collect()
}

pub struct Build<'a> {
    pub instance: &'a Instance,
    pub request: &'a ExportRequest,
    pub id: String,
    pub mods: Vec<ModEntry>,
    pub exported_by: &'a str,
    pub base: Option<(BaseSpec, BaseInfo)>,
    pub delete: Vec<String>,
}

/// The `cast.json` of an export, without the embedded files: the writer adds those with their hashes.
pub fn build(build: Build<'_>) -> CastFile {
    let instance = build.instance;
    let request = build.request;

    let loader = match instance.loader {
        LoaderType::Vanilla => None,
        loader => Some(LoaderSpec {
            loader,
            version: instance.loader_version.clone(),
        }),
    };

    let mut mods = build.mods;
    mods.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.version_id.cmp(&b.version_id))
    });

    let mut delete = build.delete;
    delete.sort();
    delete.dedup();

    let (base, base_info) = match build.base {
        Some((spec, info)) => (Some(spec), Some(info)),
        None => (None, None),
    };

    CastFile {
        format: FORMAT_VERSION,
        exported_by: build.exported_by.to_string(),
        exported_at: now_millis(),
        author: request.author.clone(),
        description: request.description.clone(),
        base_info,
        manifest: Manifest {
            schema_version: SCHEMA_VERSION,
            id: build.id,
            name: request.name.clone(),
            version: request.version.clone(),
            changelog: request.changelog.clone(),
            minecraft: instance.minecraft_version.clone(),
            loader,
            base,
            mods,
            files: Vec::new(),
            delete,
            settings: PackSettings {
                recommended_ram: request.recommended_ram,
            },
        },
    }
}

pub struct Split {
    pub mods: Vec<ModEntry>,
    pub embedded: Vec<ExportFile>,
}

/// Files the catalogs know become links, the rest travels inside the file.
pub fn split(files: Vec<ExportFile>, lookup: &Lookup) -> CommandResult<Split> {
    let mut mods = Vec::new();
    let mut embedded = Vec::new();

    for file in files {
        match lookup.found.get(&file.key) {
            Some(found) => mods.push(entry(&file.key, found)),
            None => embedded.push(file),
        }
    }

    if mods.len() > MAX_FILE_ENTRIES || embedded.len() > MAX_FILE_ENTRIES {
        return Err(
            CommandError::invalid_input("error.reason.cast.too_many_files")
                .param("count", mods.len().max(embedded.len()))
                .param("limit", MAX_FILE_ENTRIES),
        );
    }

    let size: u64 = embedded.iter().map(|file| file.size).sum();

    if size > super::file::MAX_UNPACKED {
        return Err(CommandError::invalid_input("error.reason.cast.too_large")
            .param("size", size.div_ceil(1024 * 1024))
            .param("limit", super::file::MAX_UNPACKED / 1024 / 1024));
    }

    Ok(Split { mods, embedded })
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::download::DownloadTask;
    use crate::packs::BlockedFile;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(dir: &Path, key: &str, bytes: &[u8]) {
        let path = dir.join(key);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn matched(provider: PackProvider) -> CatalogMatch {
        CatalogMatch {
            provider,
            project_id: "AANobbMI".into(),
            version_id: "abc".into(),
            version_number: "1.0".into(),
            title: "Sodium".into(),
            slug: "sodium".into(),
            icon_url: String::new(),
            page_url: String::new(),
            authors: Vec::new(),
        }
    }

    fn hashes(key: &str, sha1: &str) -> BTreeMap<String, FileHashes> {
        BTreeMap::from([(
            key.to_string(),
            FileHashes {
                sha1: sha1.into(),
                fingerprint: None,
            },
        )])
    }

    fn task(key: &str, sha1: Option<&str>) -> (String, DownloadTask) {
        (
            key.to_string(),
            DownloadTask::verified(
                "https://x",
                PathBuf::from(key),
                None,
                sha1.map(str::to_string),
            ),
        )
    }

    #[test]
    fn personal_and_cached_things_stay_home_by_default() {
        for key in [
            "saves",
            "screenshots",
            "logs",
            "servers.dat",
            "essential",
            ".cache",
            "journeymap",
            "latest.log",
        ] {
            let choice = choice(key);
            assert!(!choice.selected, "{key} is picked by default");
            assert!(choice.note.is_some(), "{key} has no reason");
        }

        for key in [
            "mods",
            "config",
            "kubejs",
            "defaultconfigs",
            "resourcepacks",
            "options.txt",
        ] {
            assert!(choice(key).selected, "{key} is left out by default");
        }
    }

    #[test]
    fn launcher_files_and_junk_never_show_up() {
        for key in [
            "natives",
            "client.jar",
            "config/Thumbs.db",
            "mods/.sodium.jar.0f3a.tmp",
        ] {
            assert!(choice(key).hidden, "{key} is shown");
        }

        assert!(!choice("config/.hidden-but-fine").hidden);
    }

    #[test]
    fn game_settings_only_land_where_there_are_none() {
        assert_eq!(choice("options.txt").mode, FileMode::Once);
        assert_eq!(choice("optionsshaders.txt").mode, FileMode::Once);
        assert_eq!(choice("config/options.txt").mode, FileMode::Always);
        assert_eq!(choice("config/sodium.json").mode, FileMode::Always);
    }

    #[test]
    fn programs_are_marked_and_left_out() {
        let choice = choice("config/tool.exe");

        assert!(!choice.selected);
        assert_eq!(choice.note, Some(Note::Forbidden));
    }

    #[test]
    fn the_tree_counts_sizes_and_follows_the_parent_choice() {
        let dir = temp_dir();
        put(&dir, "config/a.toml", b"12345");
        put(&dir, "config/deep/b.toml", b"123");
        put(&dir, "saves/World/level.dat", b"1");
        put(&dir, "options.txt", b"fov");
        put(&dir, "client.jar", b"jar");
        put(&dir, "natives/lwjgl.dll", b"dll");

        let tree = scan_tree(&dir).unwrap();
        let keys: Vec<_> = tree.iter().map(|entry| entry.key.as_str()).collect();

        assert_eq!(keys, vec!["config", "saves", "options.txt"]);

        let config = &tree[0];
        assert_eq!(config.size, 8);
        assert_eq!(config.files, 2);
        assert!(config.children.iter().all(|child| child.selected));

        let saves = &tree[1];
        assert!(!saves.selected);
        assert!(
            saves.children.iter().all(|child| !child.selected),
            "a world inside a skipped folder is skipped too"
        );

        assert_eq!(tree[2].mode, FileMode::Once);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn collecting_expands_folders_and_skips_programs() {
        let dir = temp_dir();
        put(&dir, "config/a.toml", b"a");
        put(&dir, "config/nested/b.toml", b"b");
        put(&dir, "config/setup.exe", b"MZ");
        put(&dir, "options.txt", b"fov");
        put(&dir, "mods/sodium.jar", b"jar");

        let collected = collect(
            &dir,
            &[
                "config".into(),
                "options.txt".into(),
                "config/a.toml".into(),
                "gone".into(),
            ],
        )
        .unwrap();

        let keys: Vec<_> = collected
            .files
            .iter()
            .map(|file| file.key.as_str())
            .collect();
        assert_eq!(
            keys,
            vec!["config/a.toml", "config/nested/b.toml", "options.txt"]
        );
        assert_eq!(collected.skipped, vec!["config/setup.exe"]);
        assert_eq!(collected.files[2].mode, FileMode::Once);

        assert!(collect(&dir, &["../outside".into()]).is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn only_direct_children_of_catalog_folders_can_be_links() {
        assert!(is_catalog_candidate("mods/sodium.jar"));
        assert!(is_catalog_candidate("resourcepacks/Faithful.zip"));
        assert!(!is_catalog_candidate("mods/1.20/sodium.jar"));
        assert!(!is_catalog_candidate("config/sodium.jar"));
        assert!(!is_catalog_candidate("mods"));
    }

    #[test]
    fn a_file_the_catalog_resolves_to_the_same_bytes_stays_a_link() {
        let found = BTreeMap::from([(
            "mods/sodium.jar".to_string(),
            matched(PackProvider::Modrinth),
        )]);
        let resolved = ResolvedMods {
            files: vec![task("mods/sodium-renamed.jar", Some("ABC"))],
            blocked: Vec::new(),
        };

        let (kept, targets) = verified(&found, &hashes("mods/sodium.jar", "abc"), &resolved);
        assert_eq!(kept.len(), 1, "a renamed file is still the same file");
        assert_eq!(targets["mods/sodium.jar"], "mods/sodium-renamed.jar");
    }

    #[test]
    fn a_file_the_catalog_puts_elsewhere_is_embedded() {
        let found = BTreeMap::from([(
            "resourcepacks/pack.zip".to_string(),
            matched(PackProvider::Modrinth),
        )]);
        let resolved = ResolvedMods {
            files: vec![task("datapacks/pack.zip", Some("abc"))],
            blocked: Vec::new(),
        };

        assert!(
            verified(&found, &hashes("resourcepacks/pack.zip", "abc"), &resolved)
                .0
                .is_empty()
        );
    }

    #[test]
    fn a_different_primary_file_is_embedded() {
        let found = BTreeMap::from([(
            "mods/sodium.jar".to_string(),
            matched(PackProvider::Modrinth),
        )]);
        let resolved = ResolvedMods {
            files: vec![task("mods/sodium.jar", Some("other"))],
            blocked: Vec::new(),
        };

        assert!(
            verified(&found, &hashes("mods/sodium.jar", "abc"), &resolved)
                .0
                .is_empty()
        );
    }

    #[test]
    fn a_curseforge_file_without_a_download_link_stays_a_link() {
        let found = BTreeMap::from([(
            "mods/entityculling.jar.disabled".to_string(),
            matched(PackProvider::CurseForge),
        )]);
        let resolved = ResolvedMods {
            files: Vec::new(),
            blocked: vec![BlockedFile {
                file_name: "entityculling.jar".into(),
                target_path: "mods/entityculling.jar.disabled".into(),
                website_url: String::new(),
                sha1: None,
                local_path: None,
            }],
        };

        let (kept, _) = verified(
            &found,
            &hashes("mods/entityculling.jar.disabled", "abc"),
            &resolved,
        );
        assert_eq!(kept.len(), 1);
        assert!(
            entry(
                "mods/entityculling.jar.disabled",
                &kept["mods/entityculling.jar.disabled"]
            )
            .optional
        );
    }

    #[test]
    fn links_and_embedded_files_are_split() {
        let lookup = Lookup {
            found: BTreeMap::from([(
                "mods/sodium.jar".to_string(),
                matched(PackProvider::Modrinth),
            )]),
            unchecked: BTreeSet::from(["mods/mystery.jar".to_string()]),
            ..Default::default()
        };

        let file = |key: &str| ExportFile {
            key: key.into(),
            path: PathBuf::from(key),
            size: 1,
            mode: FileMode::Always,
        };

        let split = split(
            vec![
                file("mods/sodium.jar"),
                file("mods/mystery.jar"),
                file("config/a.toml"),
            ],
            &lookup,
        )
        .unwrap();

        assert_eq!(split.mods.len(), 1);
        assert_eq!(split.mods[0].provider, Some(PackProvider::Modrinth));
        assert_eq!(split.embedded.len(), 2);

        assert_eq!(lookup.source("mods/sodium.jar").kind, SourceKind::Modrinth);
        assert_eq!(
            lookup.source("mods/mystery.jar").kind,
            SourceKind::Unchecked
        );
        assert_eq!(lookup.source("config/a.toml").kind, SourceKind::Embedded);
    }

    #[test]
    fn a_request_needs_a_name_a_version_and_something_to_export() {
        let request = ExportRequest {
            name: "  Мой пак ".into(),
            version: "1.0".into(),
            include: vec!["mods".into(), "mods\\".into(), "config".into()],
            recommended_ram: Some(0),
            ..Default::default()
        }
        .normalized()
        .unwrap();

        assert_eq!(request.name, "Мой пак");
        assert_eq!(request.include, vec!["mods", "config"]);
        assert_eq!(request.recommended_ram, None);

        for broken in [
            ExportRequest {
                version: "1".into(),
                include: vec!["mods".into()],
                ..Default::default()
            },
            ExportRequest {
                name: "a".into(),
                include: vec!["mods".into()],
                ..Default::default()
            },
            ExportRequest {
                name: "a".into(),
                version: "1".into(),
                ..Default::default()
            },
            ExportRequest {
                name: "a".into(),
                version: "1".into(),
                include: vec!["../x".into()],
                ..Default::default()
            },
        ] {
            assert!(broken.normalized().is_err());
        }
    }

    fn instance(extra: serde_json::Value) -> Instance {
        let mut value = serde_json::json!({
            "id": "abc",
            "name": "Сборка",
            "description": "Описание",
            "minecraftVersion": "1.20.1",
            "type": "fabric",
            "loaderVersion": "0.16.5"
        });

        for (key, field) in extra.as_object().unwrap() {
            value[key] = field.clone();
        }

        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn the_pack_keeps_its_id_across_exports() {
        assert_eq!(pack_id(&instance(serde_json::json!({}))), None);

        let imported = instance(serde_json::json!({
            "castpack": {"origin": "file", "catalogId": "c1f0", "manifestUrl": "", "version": "1.2"}
        }));
        assert_eq!(pack_id(&imported).as_deref(), Some("c1f0"));
        assert_eq!(defaults(&imported).version, "1.2");
        assert!(defaults(&imported).known_pack);

        let exported = instance(serde_json::json!({
            "castExport": {"id": "mine", "version": "2.0", "author": "zaralX"}
        }));
        assert_eq!(pack_id(&exported).as_deref(), Some("mine"));
        assert_eq!(defaults(&exported).author, "zaralX");

        let catalog = instance(serde_json::json!({
            "castpack": {"catalogId": "rpg", "manifestUrl": "https://x"}
        }));
        assert_eq!(
            pack_id(&catalog),
            None,
            "an export of a catalog pack is a new pack"
        );
    }

    #[test]
    fn a_built_manifest_passes_its_own_validation() {
        let instance = instance(serde_json::json!({}));
        let request = ExportRequest {
            name: "Мой пак".into(),
            version: "1.0".into(),
            recommended_ram: Some(6144),
            include: vec!["mods".into()],
            ..Default::default()
        };

        let file = build(Build {
            instance: &instance,
            request: &request,
            id: "c1f0".into(),
            mods: vec![entry("mods/sodium.jar", &matched(PackProvider::Modrinth))],
            exported_by: "1.6.0",
            base: None,
            delete: Vec::new(),
        });

        file.validate().unwrap();
        assert_eq!(
            file.manifest.loader.as_ref().unwrap().loader,
            LoaderType::Fabric
        );
        assert_eq!(file.manifest.settings.recommended_ram, Some(6144));

        let vanilla = self::instance(serde_json::json!({"type": "vanilla"}));
        let file = build(Build {
            instance: &vanilla,
            request: &request,
            id: "c1f0".into(),
            mods: Vec::new(),
            exported_by: "1.6.0",
            base: None,
            delete: Vec::new(),
        });
        assert!(file.manifest.loader.is_none());
    }

    #[test]
    fn a_pack_on_a_base_lists_the_base_and_what_it_takes_away() {
        let instance = instance(serde_json::json!({}));
        let request = ExportRequest {
            name: "Мой пак".into(),
            version: "1.0".into(),
            include: vec!["mods".into()],
            ..Default::default()
        };

        let file = build(Build {
            instance: &instance,
            request: &request,
            id: "c1f0".into(),
            mods: Vec::new(),
            exported_by: "1.6.0",
            base: Some((
                BaseSpec {
                    provider: PackProvider::Modrinth,
                    project_id: "1KVo5zza".into(),
                    version_id: "abc".into(),
                },
                BaseInfo {
                    name: "Fabulously Optimized".into(),
                    version: "6.2.0".into(),
                },
            )),
            delete: vec![
                "mods/b.jar".into(),
                "mods/a.jar".into(),
                "mods/a.jar".into(),
            ],
        });

        file.validate().unwrap();
        assert_eq!(file.manifest.delete, vec!["mods/a.jar", "mods/b.jar"]);
        assert_eq!(file.manifest.base.unwrap().project_id, "1KVo5zza");
        assert_eq!(file.base_info.unwrap().name, "Fabulously Optimized");
    }

    fn sha(value: &str) -> String {
        value.to_string()
    }

    #[test]
    fn only_the_difference_with_the_base_travels() {
        let base: BaseFiles = BTreeMap::from([
            ("mods/same.jar".to_string(), Some(sha("AAA"))),
            ("mods/gone.jar".to_string(), Some(sha("bbb"))),
            ("mods/edited.jar".to_string(), Some(sha("ccc"))),
            ("mods/switched-off.jar".to_string(), Some(sha("ddd"))),
            ("mods/no-hash.jar".to_string(), None),
            ("config/base.toml".to_string(), Some(sha("eee"))),
        ]);

        let local = BTreeMap::from([
            ("mods/same.jar".to_string(), sha("aaa")),
            ("mods/edited.jar".to_string(), sha("xxx")),
            ("mods/switched-off.jar.disabled".to_string(), sha("ddd")),
            ("mods/no-hash.jar".to_string(), sha("anything")),
        ]);

        let include = vec!["mods".to_string()];
        let diff = diff(&base, &local, covered_by(&include));

        assert_eq!(
            diff.skip,
            BTreeSet::from(["mods/same.jar".to_string(), "mods/no-hash.jar".to_string()])
        );
        assert_eq!(
            diff.delete,
            BTreeSet::from([
                "mods/gone.jar".to_string(),
                "mods/switched-off.jar".to_string()
            ])
        );
        assert_eq!(
            diff.replaced,
            BTreeSet::from(["mods/edited.jar".to_string()])
        );
    }

    #[test]
    fn a_folder_left_out_of_the_export_stays_as_the_base_has_it() {
        let base: BaseFiles = BTreeMap::from([("config/base.toml".to_string(), Some(sha("eee")))]);
        let include = vec!["mods/sodium.jar".to_string(), "options.txt".to_string()];

        assert_eq!(
            diff(&base, &BTreeMap::new(), covered_by(&include)),
            Diff::default()
        );

        let include = vec!["config/other.toml".to_string()];
        let diff = diff(&base, &BTreeMap::new(), covered_by(&include));
        assert!(
            diff.delete.contains("config/base.toml"),
            "an unticked file of a covered folder is taken away"
        );
    }

    #[test]
    fn a_replaced_file_that_lands_under_another_name_takes_the_old_one_away() {
        let diff = Diff {
            replaced: BTreeSet::from(["mods/a.jar".to_string(), "mods/b.jar".to_string()]),
            ..Default::default()
        };
        let lookup = Lookup {
            targets: BTreeMap::from([
                ("mods/a.jar".to_string(), "mods/a-1.1.jar".to_string()),
                ("mods/b.jar".to_string(), "mods/b.jar".to_string()),
            ]),
            ..Default::default()
        };

        assert_eq!(stale_replacements(&diff, &lookup), vec!["mods/a.jar"]);
    }

    #[test]
    fn files_the_base_brings_are_marked_in_the_tree() {
        let leaf = |key: &str| TreeEntry {
            key: key.into(),
            name: key.into(),
            dir: false,
            size: 1,
            files: 1,
            selected: true,
            mode: FileMode::Always,
            note: None,
            source: None,
            in_base: false,
            children: Vec::new(),
        };

        let mut tree = vec![TreeEntry {
            dir: true,
            children: vec![leaf("mods/same.jar"), leaf("mods/own.jar")],
            ..leaf("mods")
        }];

        mark_base(&mut tree, &BTreeSet::from(["mods/same.jar".to_string()]));

        assert!(tree[0].children[0].in_base);
        assert!(!tree[0].children[1].in_base);

        let base: BaseFiles = BTreeMap::from([("mods/own.jar".to_string(), None)]);
        let found = base_candidates(&tree, &base, Path::new("/mc"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "mods/own.jar");
    }
}
