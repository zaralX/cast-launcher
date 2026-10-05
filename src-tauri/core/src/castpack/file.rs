use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use crate::error::{CommandError, CommandResult};
use crate::fs_util::safe_join;
use crate::instance::{LoaderType, PackProvider};

use super::manifest::{json_error, EmbeddedFile, FileEntry, FileMode, Origin, MAX_FILE_ENTRIES};
use super::Manifest;

pub const FORMAT_VERSION: u32 = 1;

pub const EXTENSION: &str = "cast";

pub const MANIFEST_ENTRY: &str = "cast.json";

const ICON_STEM: &str = "icon";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    pub extension: String,
    pub bytes: Vec<u8>,
}

const FILES_DIR: &str = "files";

const MAX_MANIFEST: u64 = 16 * 1024 * 1024;

pub const MAX_UNPACKED: u64 = 4 * 1024 * 1024 * 1024;

/// Programs Windows runs on a double click: a pack has no reason to carry them.
const FORBIDDEN_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "bat", "cmd", "com", "ps1", "psm1", "vbs", "vbe", "wsf", "wsh",
    "lnk", "scr", "msi", "msp", "sh", "reg", "hta", "jse", "pif", "cpl",
];

/// Already compressed formats: deflating them again only costs time.
const STORED_EXTENSIONS: &[&str] = &[
    "jar", "zip", "litemod", "png", "jpg", "jpeg", "webp", "gif", "ogg", "mp3", "mp4", "gz", "xz",
    "7z", "rar", "mrpack",
];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseInfo {
    pub name: String,
    pub version: String,
}

/// What `cast.json` holds: the shared CastPack manifest wrapped in what only a file needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastFile {
    pub format: u32,
    #[serde(default)]
    pub exported_by: String,
    #[serde(default)]
    pub exported_at: u64,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_info: Option<BaseInfo>,
    pub manifest: Manifest,
}

impl CastFile {
    pub fn parse(bytes: &[u8]) -> CommandResult<Self> {
        let file: Self = serde_json::from_slice(bytes)
            .map_err(|e| json_error(e, "error.reason.cast.manifest_corrupted"))?;

        file.validate()?;

        Ok(file)
    }

    pub fn validate(&self) -> CommandResult<()> {
        if self.format == 0 || self.format > FORMAT_VERSION {
            return Err(
                CommandError::unsupported("error.reason.cast.format_version")
                    .param("version", self.format)
                    .param("supported", FORMAT_VERSION),
            );
        }

        self.manifest.validate(Origin::File)?;

        for file in self.manifest.embedded_files()? {
            if forbidden(&file.key) {
                return Err(
                    CommandError::invalid_input("error.reason.cast.forbidden_file")
                        .param("path", &file.key),
                );
            }
        }

        Ok(())
    }

    pub fn embedded_size(&self) -> u64 {
        self.manifest
            .files
            .iter()
            .filter(|file| file.embedded)
            .filter_map(|file| file.size)
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CastBase {
    pub provider: PackProvider,
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    pub version: String,
}

/// An instance that already holds a pack with the same id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingPack {
    pub instance_id: String,
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: LoaderType,
}

/// What the import dialog shows about a `.cast` before anything is installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CastPreview {
    pub id: String,
    pub changelog: String,
    pub exported_by: String,
    pub exported_at: u64,
    pub modrinth_mods: usize,
    pub curseforge_mods: usize,
    pub linked_files: usize,
    pub embedded_files: usize,
    pub embedded_size: u64,
    /// Embedded mods: code that came from neither Modrinth nor CurseForge.
    pub embedded_code: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<CastBase>,
    pub has_icon: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing: Option<ExistingPack>,
}

impl CastFile {
    pub fn preview(&self, has_icon: bool) -> CastPreview {
        let manifest = &self.manifest;

        let count = |provider: PackProvider| {
            manifest
                .mods
                .iter()
                .filter(|entry| entry.provider == Some(provider))
                .count()
        };

        let embedded: Vec<&FileEntry> =
            manifest.files.iter().filter(|file| file.embedded).collect();

        CastPreview {
            id: manifest.id.clone(),
            changelog: manifest.changelog.clone(),
            exported_by: self.exported_by.clone(),
            exported_at: self.exported_at,
            modrinth_mods: count(PackProvider::Modrinth),
            curseforge_mods: count(PackProvider::CurseForge),
            linked_files: manifest
                .mods
                .iter()
                .filter(|entry| entry.provider.is_none())
                .count()
                + manifest.files.len()
                - embedded.len(),
            embedded_files: embedded.len(),
            embedded_size: self.embedded_size(),
            embedded_code: embedded
                .iter()
                .filter(|file| is_code(&file.path))
                .map(|file| file.path.clone())
                .collect(),
            base: manifest.base.as_ref().map(|base| {
                let info = self.base_info.clone().unwrap_or_default();

                CastBase {
                    provider: base.provider,
                    project_id: base.project_id.clone(),
                    version_id: base.version_id.clone(),
                    name: info.name,
                    version: info.version,
                }
            }),
            has_icon,
            existing: None,
        }
    }

    pub fn needs_curseforge(&self) -> bool {
        self.manifest
            .base
            .as_ref()
            .is_some_and(|base| base.provider == PackProvider::CurseForge)
            || self
                .manifest
                .mods
                .iter()
                .any(|entry| entry.provider == Some(PackProvider::CurseForge))
    }
}

fn is_code(key: &str) -> bool {
    let in_mods = key.starts_with(&format!("{}/", crate::mods::FOLDER));
    let key = key
        .strip_suffix(crate::mods::DISABLED_SUFFIX)
        .unwrap_or(key);

    match extension(key).as_deref() {
        Some("jar" | "litemod") => true,
        Some("zip") => in_mods,
        _ => false,
    }
}

/// `.cast` files among the arguments the system started the launcher with.
pub fn opened_paths(args: &[String], cwd: Option<&Path>) -> Vec<String> {
    args.iter()
        .map(|arg| Path::new(arg.trim()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION))
        })
        .map(|path| match (path.is_relative(), cwd) {
            (true, Some(cwd)) => cwd.join(path),
            _ => path.to_path_buf(),
        })
        .map(|path| path.display().to_string())
        .collect()
}

pub fn forbidden(key: &str) -> bool {
    let key = key
        .strip_suffix(crate::mods::DISABLED_SUFFIX)
        .unwrap_or(key);

    extension(key).is_some_and(|extension| FORBIDDEN_EXTENSIONS.contains(&extension.as_str()))
}

fn extension(key: &str) -> Option<String> {
    let name = key.rsplit('/').next()?;
    let (_, extension) = name.rsplit_once('.')?;

    Some(extension.to_ascii_lowercase())
}

fn entry_name(key: &str) -> String {
    format!("{FILES_DIR}/{key}")
}

/// A name for the saved file that every file system accepts.
pub fn file_name(name: &str, version: &str) -> String {
    let raw = match version.trim() {
        "" => name.trim().to_string(),
        version => format!("{}-{version}", name.trim()),
    };

    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .take(80)
        .collect();

    let cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_string();

    match cleaned.is_empty() {
        true => format!("pack.{EXTENSION}"),
        false => format!("{cleaned}.{EXTENSION}"),
    }
}

#[derive(Debug)]
pub struct Opened {
    pub file: CastFile,
    pub size: u64,
    pub icon: Option<Icon>,
}

pub async fn open(path: &Path) -> CommandResult<Opened> {
    let path = path.to_path_buf();

    tokio::task::spawn_blocking(move || open_blocking(&path))
        .await
        .map_err(|e| CommandError::task_panicked("read_cast_file", e))?
}

pub fn is_cast(names: &std::collections::BTreeSet<String>) -> bool {
    names.contains(MANIFEST_ENTRY)
}

pub fn open_blocking(path: &Path) -> CommandResult<Opened> {
    let mut archive = crate::archive::open(path)?;

    if archive.len() > MAX_FILE_ENTRIES * 2 + 8 {
        return Err(
            CommandError::invalid_input("error.reason.cast.too_many_entries")
                .param("count", archive.len()),
        );
    }

    let bytes = {
        let mut entry = archive.by_name(MANIFEST_ENTRY).map_err(|e| {
            CommandError::invalid_input("error.reason.cast.not_cast").with_details(e.to_string())
        })?;

        if entry.size() > MAX_MANIFEST {
            return Err(
                CommandError::archive("error.reason.archive.entry_too_large")
                    .param("entry", MANIFEST_ENTRY),
            );
        }

        read_limited(&mut entry, MAX_MANIFEST, MANIFEST_ENTRY)?
    };

    let file = CastFile::parse(&bytes)?;

    let mut unpacked = 0u64;

    for embedded in file.manifest.embedded_files()? {
        let name = entry_name(&embedded.key);

        let entry = archive.by_name(&name).map_err(|e| {
            CommandError::archive("error.reason.cast.embedded_missing")
                .param("path", &embedded.key)
                .with_details(e.to_string())
        })?;

        if embedded.size.is_some_and(|size| size != entry.size()) {
            return Err(CommandError::archive("error.reason.cast.embedded_size")
                .param("path", &embedded.key)
                .with_details(format!(
                    "manifest {:?}, archive {}",
                    embedded.size,
                    entry.size()
                )));
        }

        unpacked = unpacked.saturating_add(entry.size());
    }

    if unpacked > MAX_UNPACKED {
        return Err(CommandError::invalid_input("error.reason.cast.too_large")
            .param("size", megabytes(unpacked))
            .param("limit", megabytes(MAX_UNPACKED)));
    }

    let icon = read_icon(&mut archive);

    Ok(Opened {
        file,
        size: std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0),
        icon,
    })
}

fn megabytes(bytes: u64) -> u64 {
    bytes.div_ceil(1024 * 1024)
}

fn read_icon(archive: &mut ZipArchive<File>) -> Option<Icon> {
    for extension in crate::icons::extensions() {
        let name = format!("{ICON_STEM}.{extension}");

        let Ok(mut entry) = archive.by_name(&name) else {
            continue;
        };

        if entry.size() > crate::icons::MAX_SIZE {
            log::warn!("The icon inside the .cast file is too large, skipping it");
            return None;
        }

        return read_limited(&mut entry, crate::icons::MAX_SIZE, &name)
            .ok()
            .map(|bytes| Icon {
                extension: extension.to_string(),
                bytes,
            });
    }

    None
}

fn read_limited(reader: &mut impl Read, limit: u64, entry: &str) -> CommandResult<Vec<u8>> {
    let mut bytes = Vec::new();

    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| {
            CommandError::archive("error.reason.archive.read_file")
                .param("entry", entry)
                .with_details(e.to_string())
        })?;

    if bytes.len() as u64 > limit {
        return Err(
            CommandError::archive("error.reason.archive.entry_too_large").param("entry", entry),
        );
    }

    Ok(bytes)
}

/// Unpacks the listed files and nothing else, checking each against its hash. A file already
/// in place with the same hash is left alone, `skip` turns down a target before that.
pub async fn extract<F>(
    archive: PathBuf,
    files: Vec<EmbeddedFile>,
    minecraft_dir: PathBuf,
    skip: F,
) -> CommandResult<Vec<String>>
where
    F: Fn(&Path) -> bool + Send + 'static,
{
    tokio::task::spawn_blocking(move || extract_blocking(&archive, &files, &minecraft_dir, skip))
        .await
        .map_err(|e| CommandError::task_panicked("extract_cast_files", e))?
}

fn extract_blocking(
    archive_path: &Path,
    files: &[EmbeddedFile],
    minecraft_dir: &Path,
    skip: impl Fn(&Path) -> bool,
) -> CommandResult<Vec<String>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let mut archive = crate::archive::open(archive_path)?;
    let mut written = Vec::new();

    for file in files {
        let target = safe_join(minecraft_dir, &file.key)?;

        if skip(&target) || same_file(&target, file) {
            continue;
        }

        let mut entry = archive.by_name(&entry_name(&file.key)).map_err(|e| {
            CommandError::archive("error.reason.cast.embedded_missing")
                .param("path", &file.key)
                .with_details(e.to_string())
        })?;

        let declared = file.size.unwrap_or(entry.size());
        let mut limited = (&mut entry).take(declared + 1);

        write_checked(&mut limited, &target, file, declared)?;
        written.push(file.key.clone());
    }

    Ok(written)
}

fn same_file(target: &Path, file: &EmbeddedFile) -> bool {
    let Ok(meta) = std::fs::metadata(target) else {
        return false;
    };

    if file.size.is_some_and(|size| size != meta.len()) {
        return false;
    }

    File::open(target)
        .ok()
        .and_then(|mut opened| sha1_of(&mut opened).ok())
        .is_some_and(|sha1| sha1 == file.sha1)
}

fn write_checked(
    reader: &mut impl Read,
    target: &Path,
    file: &EmbeddedFile,
    declared: u64,
) -> CommandResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CommandError::io("error.reason.fs.create_dir", parent, e))?;
    }

    let temp = temp_sibling(target);

    let outcome = (|| {
        let mut out = File::create(&temp)
            .map_err(|e| CommandError::io("error.reason.fs.create_file", &temp, e))?;

        let mut hashing = HashingWriter::new(&mut out);

        io::copy(reader, &mut hashing)
            .map_err(|e| CommandError::io("error.reason.fs.extract", target, e))?;

        let (sha1, size) = hashing.finish();

        if size != declared || sha1 != file.sha1 {
            return Err(
                CommandError::hash_mismatch("error.reason.cast.embedded_hash")
                    .param("path", &file.key)
                    .with_details(format!(
                        "expected {} ({declared} bytes), got {sha1} ({size} bytes)",
                        file.sha1
                    )),
            );
        }

        out.sync_all()
            .map_err(|e| CommandError::io("error.reason.fs.write_file", &temp, e))?;

        Ok(())
    })();

    if let Err(error) = outcome {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }

    std::fs::rename(&temp, target).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        CommandError::io("error.reason.fs.write_file", target, e)
    })
}

fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    path.with_file_name(format!(".{name}.{}.tmp", uuid::Uuid::new_v4().simple()))
}

struct HashingWriter<W> {
    inner: W,
    hasher: Sha1,
    size: u64,
}

impl<W: Write> HashingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha1::new(),
            size: 0,
        }
    }

    fn finish(self) -> (String, u64) {
        (hex(&self.hasher.finalize()), self.size)
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        self.size += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn sha1_of(reader: &mut impl Read) -> io::Result<String> {
    let mut hashing = HashingWriter::new(io::sink());
    io::copy(reader, &mut hashing)?;
    Ok(hashing.finish().0)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A file to put inside the archive.
#[derive(Debug, Clone)]
pub struct Embed {
    pub key: String,
    pub source: PathBuf,
    pub mode: FileMode,
}

pub struct WriteOptions<'a> {
    pub icon: Option<&'a Icon>,
    pub progress: &'a dyn Fn(u64),
    pub cancelled: &'a dyn Fn() -> bool,
}

/// Writes a `.cast` file: embedded files sorted by path with fixed dates, so the same pack
/// gives the same bytes, then the icon, then `cast.json` with the hashes it collected.
pub fn write(
    target: &Path,
    mut file: CastFile,
    mut embeds: Vec<Embed>,
    options: WriteOptions<'_>,
) -> CommandResult<CastFile> {
    embeds.sort_by(|a, b| a.key.cmp(&b.key));
    file.manifest.files.retain(|entry| !entry.embedded);

    let out = File::create(target)
        .map_err(|e| CommandError::io("error.reason.fs.create_file", target, e))?;
    let mut zip = ZipWriter::new(out);
    let mut written = 0u64;

    for embed in &embeds {
        if (options.cancelled)() {
            return Err(CommandError::aborted("error.reason.cast.export_cancelled"));
        }

        let mut source = File::open(&embed.source)
            .map_err(|e| CommandError::io("error.reason.fs.read_file", &embed.source, e))?;
        let size = source
            .metadata()
            .map_err(|e| CommandError::io("error.reason.fs.read_file", &embed.source, e))?
            .len();

        zip.start_file(entry_name(&embed.key), entry_options(&embed.key, size))
            .map_err(|e| zip_error(target, e))?;

        let mut hashing = HashingWriter::new(&mut zip);
        let mut chunk = vec![0u8; 256 * 1024];

        loop {
            let read = source
                .read(&mut chunk)
                .map_err(|e| CommandError::io("error.reason.fs.read_file", &embed.source, e))?;

            if read == 0 {
                break;
            }

            hashing
                .write_all(&chunk[..read])
                .map_err(|e| CommandError::io("error.reason.fs.write_file", target, e))?;

            written += read as u64;
            (options.progress)(written);
        }

        let (sha1, size) = hashing.finish();

        file.manifest.files.push(FileEntry {
            path: embed.key.clone(),
            url: String::new(),
            embedded: true,
            sha1: Some(sha1),
            size: Some(size),
            mode: embed.mode,
        });
    }

    file.manifest.files.sort_by(|a, b| a.path.cmp(&b.path));
    file.validate()?;

    if let Some(icon) = options.icon {
        let name = format!("{ICON_STEM}.{}", icon.extension.to_ascii_lowercase());

        zip.start_file(&name, entry_options(&name, icon.bytes.len() as u64))
            .map_err(|e| zip_error(target, e))?;
        zip.write_all(&icon.bytes)
            .map_err(|e| CommandError::io("error.reason.fs.write_file", target, e))?;
    }

    let manifest = serde_json::to_vec_pretty(&file).map_err(|e| {
        CommandError::unknown("error.reason.internal.serialize").with_details(e.to_string())
    })?;

    zip.start_file(
        MANIFEST_ENTRY,
        entry_options(MANIFEST_ENTRY, manifest.len() as u64),
    )
    .map_err(|e| zip_error(target, e))?;
    zip.write_all(&manifest)
        .map_err(|e| CommandError::io("error.reason.fs.write_file", target, e))?;

    let mut out = zip.finish().map_err(|e| zip_error(target, e))?;
    out.flush()
        .map_err(|e| CommandError::io("error.reason.fs.write_file", target, e))?;

    Ok(file)
}

fn entry_options(key: &str, size: u64) -> SimpleFileOptions {
    let stored = extension(key).is_some_and(|ext| STORED_EXTENSIONS.contains(&ext.as_str()));

    SimpleFileOptions::default()
        .compression_method(match stored {
            true => CompressionMethod::Stored,
            false => CompressionMethod::Deflated,
        })
        .last_modified_time(DateTime::default())
        .unix_permissions(0o644)
        .large_file(size >= u32::MAX as u64)
}

fn zip_error(target: &Path, error: zip::result::ZipError) -> CommandError {
    CommandError::fs("error.reason.cast.write_failed")
        .param("path", target.display())
        .with_details(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use serde_json::json;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-file-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cast(extra: serde_json::Value) -> CastFile {
        let mut manifest = json!({
            "schemaVersion": super::super::SCHEMA_VERSION,
            "id": "c1f0",
            "name": "Мой пак",
            "version": "1.0.0",
            "minecraft": "1.20.1",
            "loader": {"type": "fabric", "version": "0.16.5"},
            "mods": [{"provider": "modrinth", "projectId": "AANobbMI", "versionId": "abc"}]
        });

        for (key, value) in extra.as_object().unwrap() {
            manifest[key] = value.clone();
        }

        CastFile {
            format: FORMAT_VERSION,
            exported_by: "1.6.0".into(),
            exported_at: 1,
            author: "zaralX".into(),
            description: String::new(),
            base_info: None,
            manifest: serde_json::from_value(manifest).unwrap(),
        }
    }

    fn no_progress(_: u64) {}

    fn never() -> bool {
        false
    }

    fn quiet() -> WriteOptions<'static> {
        WriteOptions {
            icon: None,
            progress: &no_progress,
            cancelled: &never,
        }
    }

    fn source(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join("source").join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn embed(dir: &Path, key: &str, bytes: &[u8], mode: FileMode) -> Embed {
        Embed {
            key: key.into(),
            source: source(dir, &key.replace('/', "_"), bytes),
            mode,
        }
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut writer = ZipWriter::new(File::create(path).unwrap());

        for (name, bytes) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }

        writer.finish().unwrap();
    }

    #[tokio::test]
    async fn a_written_pack_reads_back_and_unpacks_byte_for_byte() {
        let dir = temp_dir();
        let target = dir.join("pack.cast");

        let icon = Icon {
            extension: "WEBP".into(),
            bytes: b"webp".to_vec(),
        };

        let written = write(
            &target,
            cast(json!({})),
            vec![
                embed(&dir, "mods/private.jar", b"jar bytes", FileMode::Always),
                embed(&dir, "config/Мой конфиг.toml", b"a = 1", FileMode::Always),
                embed(&dir, "options.txt", b"fov:80", FileMode::Once),
            ],
            WriteOptions {
                icon: Some(&icon),
                ..quiet()
            },
        )
        .unwrap();

        let paths: Vec<_> = written
            .manifest
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec!["config/Мой конфиг.toml", "mods/private.jar", "options.txt"]
        );

        let opened = open(&target).await.unwrap();
        assert_eq!(opened.file, written);
        assert_eq!(
            opened.icon,
            Some(Icon {
                extension: "webp".into(),
                bytes: b"webp".to_vec(),
            })
        );
        assert_eq!(opened.file.embedded_size(), 9 + 5 + 6);

        let minecraft = dir.join("minecraft");
        let files = opened.file.manifest.embedded_files().unwrap();
        let mut unpacked = extract(target.clone(), files, minecraft.clone(), |_| false)
            .await
            .unwrap();
        unpacked.sort();

        assert_eq!(unpacked.len(), 3);
        assert_eq!(
            std::fs::read(minecraft.join("config").join("Мой конфиг.toml")).unwrap(),
            b"a = 1"
        );
        assert_eq!(
            std::fs::read(minecraft.join("mods").join("private.jar")).unwrap(),
            b"jar bytes"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_same_pack_gives_the_same_bytes() {
        let dir = temp_dir();
        let first = dir.join("first.cast");
        let second = dir.join("second.cast");

        for target in [&first, &second] {
            write(
                target,
                cast(json!({})),
                vec![
                    embed(&dir, "config/b.toml", b"b", FileMode::Always),
                    embed(&dir, "config/a.toml", b"a", FileMode::Always),
                ],
                quiet(),
            )
            .unwrap();
        }

        assert_eq!(
            std::fs::read(&first).unwrap(),
            std::fs::read(&second).unwrap()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_file_already_in_place_is_not_written_again() {
        let dir = temp_dir();
        let target = dir.join("pack.cast");

        let written = write(
            &target,
            cast(json!({})),
            vec![embed(&dir, "config/a.toml", b"same", FileMode::Always)],
            quiet(),
        )
        .unwrap();

        let minecraft = dir.join("minecraft");
        std::fs::create_dir_all(minecraft.join("config")).unwrap();
        std::fs::write(minecraft.join("config").join("a.toml"), b"same").unwrap();

        let files = written.manifest.embedded_files().unwrap();
        let unpacked = extract(target.clone(), files.clone(), minecraft.clone(), |_| false)
            .await
            .unwrap();
        assert!(unpacked.is_empty());

        std::fs::write(minecraft.join("config").join("a.toml"), b"edited").unwrap();
        let unpacked = extract(target, files, minecraft.clone(), |_| false)
            .await
            .unwrap();
        assert_eq!(unpacked, vec!["config/a.toml"]);
        assert_eq!(
            std::fs::read(minecraft.join("config").join("a.toml")).unwrap(),
            b"same"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_tampered_file_is_refused_and_leaves_nothing_behind() {
        let dir = temp_dir();
        let target = dir.join("pack.cast");

        let mut pack = cast(json!({}));
        pack.manifest.files.push(FileEntry {
            path: "mods/private.jar".into(),
            embedded: true,
            sha1: Some(hex(&Sha1::digest(b"original"))),
            size: Some(8),
            ..Default::default()
        });

        write_zip(
            &target,
            &[
                (MANIFEST_ENTRY, &serde_json::to_vec(&pack).unwrap()),
                ("files/mods/private.jar", b"tampered"),
            ],
        );

        let opened = open(&target).await.unwrap();
        let files = opened.file.manifest.embedded_files().unwrap();
        let minecraft = dir.join("minecraft");

        let error = extract(target, files, minecraft.clone(), |_| false)
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::HashMismatch);
        assert!(std::fs::read_dir(minecraft.join("mods"))
            .unwrap()
            .next()
            .is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_listed_file_missing_from_the_archive_is_reported_on_open() {
        let dir = temp_dir();
        let target = dir.join("pack.cast");

        let mut pack = cast(json!({}));
        pack.manifest.files.push(FileEntry {
            path: "config/a.toml".into(),
            embedded: true,
            sha1: Some("a".into()),
            ..Default::default()
        });

        write_zip(
            &target,
            &[(MANIFEST_ENTRY, &serde_json::to_vec(&pack).unwrap())],
        );

        let error = open(&target).await.unwrap_err();
        assert!(error.text.mentions("embedded_missing"), "{error}");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_size_that_lies_is_caught_before_unpacking() {
        let dir = temp_dir();
        let target = dir.join("pack.cast");

        let mut pack = cast(json!({}));
        pack.manifest.files.push(FileEntry {
            path: "config/a.toml".into(),
            embedded: true,
            sha1: Some("a".into()),
            size: Some(1),
            ..Default::default()
        });

        write_zip(
            &target,
            &[
                (MANIFEST_ENTRY, &serde_json::to_vec(&pack).unwrap()),
                ("files/config/a.toml", b"much more than one byte"),
            ],
        );

        let error = open(&target).await.unwrap_err();
        assert!(error.text.mentions("embedded_size"), "{error}");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn only_listed_files_leave_the_archive() {
        let dir = temp_dir();
        let target = dir.join("pack.cast");

        write(
            &target,
            cast(json!({})),
            vec![embed(&dir, "config/a.toml", b"a", FileMode::Always)],
            quiet(),
        )
        .unwrap();

        let opened = open(&target).await.unwrap();
        let minecraft = dir.join("minecraft");

        extract(
            target,
            opened.file.manifest.embedded_files().unwrap(),
            minecraft.clone(),
            |_| false,
        )
        .await
        .unwrap();

        let mut found = Vec::new();
        for entry in std::fs::read_dir(&minecraft).unwrap().flatten() {
            found.push(entry.file_name().to_string_lossy().to_string());
        }
        assert_eq!(
            found,
            vec!["config"],
            "no cast.json or icon in the game folder"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn programs_cannot_ride_inside_a_pack() {
        let dir = temp_dir();

        let error = write(
            &dir.join("pack.cast"),
            cast(json!({})),
            vec![embed(&dir, "config/setup.exe", b"MZ", FileMode::Always)],
            quiet(),
        )
        .unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(forbidden("mods/natives/LIB.DLL"));
        assert!(forbidden("mods/setup.exe.disabled"));
        assert!(!forbidden("kubejs/server_scripts/main.js"));
        assert!(!forbidden("mods/sodium.jar"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_zip_without_cast_json_is_not_a_pack() {
        let dir = temp_dir();
        let target = dir.join("pack.cast");
        write_zip(&target, &[("modrinth.index.json", b"{}")]);

        let error = open(&target).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error.text.mentions("not_cast"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_newer_format_asks_for_a_newer_launcher() {
        let mut pack = serde_json::to_value(cast(json!({}))).unwrap();
        pack["format"] = json!(FORMAT_VERSION + 1);

        let error = CastFile::parse(&serde_json::to_vec(&pack).unwrap()).unwrap_err();
        assert_eq!(error.code, ErrorCode::Unsupported);
        assert!(error.text.mentions("format_version"));
    }

    #[test]
    fn cancelling_stops_the_export() {
        let dir = temp_dir();

        let error = write(
            &dir.join("pack.cast"),
            cast(json!({})),
            vec![embed(&dir, "config/a.toml", b"a", FileMode::Always)],
            WriteOptions {
                cancelled: &|| true,
                ..quiet()
            },
        )
        .unwrap_err();

        assert!(error.is_aborted());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_preview_counts_sources_and_names_embedded_code() {
        let mut pack = cast(json!({
            "base": {"provider": "curseforge", "projectId": "925200", "versionId": "5432100"},
            "mods": [
                {"provider": "modrinth", "projectId": "a", "versionId": "b"},
                {"provider": "curseforge", "projectId": "1", "versionId": "2"},
                {"url": "https://cdn.zaralx.ru/core.jar", "path": "mods/core.jar", "sha1": "a"}
            ],
            "files": [
                {"path": "mods/private.jar", "embedded": true, "sha1": "a", "size": 10},
                {"path": "mods/old.jar.disabled", "embedded": true, "sha1": "b", "size": 5},
                {"path": "resourcepacks/pack.zip", "embedded": true, "sha1": "c", "size": 1},
                {"path": "config/a.toml", "url": "https://x/a.toml", "sha1": "d"}
            ]
        }));
        pack.base_info = Some(BaseInfo {
            name: "TerraFirmaGreg".into(),
            version: "0.10".into(),
        });

        let preview = pack.preview(true);

        assert_eq!(preview.modrinth_mods, 1);
        assert_eq!(preview.curseforge_mods, 1);
        assert_eq!(preview.linked_files, 2, "a direct mod and a linked file");
        assert_eq!(preview.embedded_files, 3);
        assert_eq!(preview.embedded_size, 16);
        assert_eq!(
            preview.embedded_code,
            vec!["mods/private.jar", "mods/old.jar.disabled"],
            "a resource pack is not code"
        );
        assert_eq!(preview.base.unwrap().name, "TerraFirmaGreg");
        assert!(pack.needs_curseforge());
    }

    #[test]
    fn only_cast_files_are_taken_from_the_arguments() {
        let cwd = Path::new("/home/player");
        let args = vec![
            "/opt/cast-launcher".to_string(),
            "--flag".to_string(),
            "Мой пак.CAST".to_string(),
            "/tmp/other.cast".to_string(),
            "pack.mrpack".to_string(),
        ];

        assert_eq!(
            opened_paths(&args, Some(cwd)),
            vec![
                cwd.join("Мой пак.CAST").display().to_string(),
                Path::new("/tmp/other.cast").display().to_string(),
            ]
        );
    }

    #[test]
    fn the_file_name_is_safe_everywhere() {
        assert_eq!(file_name("Мой пак", "1.0.0"), "Мой пак-1.0.0.cast");
        assert_eq!(file_name("a/b:c?", ""), "a_b_c_.cast");
        assert_eq!(file_name("  ...  ", ""), "pack.cast");
        assert_eq!(file_name("pack.", ""), "pack.cast");
    }
}
