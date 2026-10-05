use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::error::{CommandError, CommandResult};

pub async fn extract_natives(jar_path: PathBuf, output_dir: PathBuf) -> CommandResult<()> {
    tokio::task::spawn_blocking(move || extract_natives_blocking(&jar_path, &output_dir))
        .await
        .map_err(|e| CommandError::task_panicked("extract_natives", e))?
}

fn extract_natives_blocking(jar_path: &Path, output_dir: &Path) -> CommandResult<()> {
    let mut archive = open(jar_path)?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| CommandError::io("error.reason.fs.create_dir", output_dir, e))?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| {
            CommandError::archive("error.reason.archive.read_entry")
                .param("path", jar_path.display())
                .with_details(e.to_string())
        })?;

        let name = entry.name().to_string();

        if name.ends_with('/') || name.starts_with("META-INF/") || !is_native(&name) {
            continue;
        }

        let Some(file_name) = Path::new(&name).file_name() else {
            continue;
        };
        let out_path = output_dir.join(file_name);

        let mut out = File::create(&out_path)
            .map_err(|e| CommandError::io("error.reason.fs.create_file", &out_path, e))?;

        io::copy(&mut entry, &mut out)
            .map_err(|e| CommandError::io("error.reason.fs.extract", &out_path, e))?;
    }

    Ok(())
}

pub async fn read_entry(archive_path: PathBuf, entry: String) -> CommandResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || read_entry_blocking(&archive_path, &entry))
        .await
        .map_err(|e| CommandError::task_panicked("read_archive_entry", e))?
}

fn read_entry_blocking(archive_path: &Path, entry: &str) -> CommandResult<Vec<u8>> {
    let mut archive = open(archive_path)?;

    let mut file = archive.by_name(entry).map_err(|e| {
        CommandError::archive("error.reason.archive.missing_entry")
            .param("entry", entry)
            .param("path", archive_path.display())
            .with_details(e.to_string())
    })?;

    let mut bytes = Vec::with_capacity(file.size() as usize);

    io::copy(&mut file, &mut bytes).map_err(|e| {
        CommandError::archive("error.reason.archive.read_file")
            .param("entry", entry)
            .with_details(e.to_string())
    })?;

    Ok(bytes)
}

pub async fn extract_dir(
    archive_path: PathBuf,
    prefix: String,
    output_dir: PathBuf,
) -> CommandResult<Vec<String>> {
    extract_dir_with(archive_path, prefix, output_dir, |_, _| false).await
}

/// Files that `keep` holds on to by key or path are left as they are, but still reported as extracted.
pub async fn extract_dir_with<F>(
    archive_path: PathBuf,
    prefix: String,
    output_dir: PathBuf,
    keep: F,
) -> CommandResult<Vec<String>>
where
    F: Fn(&str, &Path) -> bool + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        extract_dir_blocking(&archive_path, &prefix, &output_dir, keep)
    })
    .await
    .map_err(|e| CommandError::task_panicked("extract_archive_dir", e))?
}

fn extract_dir_blocking(
    archive_path: &Path,
    prefix: &str,
    output_dir: &Path,
    keep: impl Fn(&str, &Path) -> bool,
) -> CommandResult<Vec<String>> {
    let mut archive = open(archive_path)?;
    let prefix = format!("{}/", prefix.trim_end_matches('/'));
    let mut extracted = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| {
            CommandError::archive("error.reason.archive.read_entry")
                .param("path", archive_path.display())
                .with_details(e.to_string())
        })?;

        let name = entry.name().to_string();

        let Some(relative) = name.strip_prefix(&prefix) else {
            continue;
        };
        if relative.is_empty() || name.ends_with('/') {
            continue;
        }

        let key = crate::fs_util::relative_key(relative)?;
        let out_path = crate::fs_util::safe_join(output_dir, &key)?;

        if keep(&key, &out_path) {
            extracted.push(key);
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CommandError::io("error.reason.fs.create_dir", parent, e))?;
        }

        let mut out = File::create(&out_path)
            .map_err(|e| CommandError::io("error.reason.fs.create_file", &out_path, e))?;

        io::copy(&mut entry, &mut out)
            .map_err(|e| CommandError::io("error.reason.fs.extract", &out_path, e))?;

        extracted.push(key);
    }

    Ok(extracted)
}

fn is_native(name: &str) -> bool {
    let lowercase = name.to_ascii_lowercase();
    [".dll", ".so", ".dylib", ".jnilib"]
        .iter()
        .any(|extension| lowercase.ends_with(extension))
        || lowercase.contains(".so.")
}

pub(crate) fn open(jar_path: &Path) -> CommandResult<ZipArchive<File>> {
    let file = File::open(jar_path)
        .map_err(|e| CommandError::io("error.reason.fs.open_archive", jar_path, e))?;

    ZipArchive::new(file).map_err(|e| {
        CommandError::archive("error.reason.archive.corrupted")
            .param("path", jar_path.display())
            .with_details(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_native_libraries() {
        assert!(is_native("windows/x64/lwjgl.dll"));
        assert!(is_native("liblwjgl.so"));
        assert!(is_native("liblwjgl.dylib"));
        assert!(is_native("liblwjgl.jnilib"));
        assert!(is_native("libfoo.so.1"));
        assert!(is_native("LWJGL.DLL"));
    }

    #[test]
    fn ignores_classes_and_metadata() {
        assert!(!is_native("org/lwjgl/Sys.class"));
        assert!(!is_native("module-info.class"));
        assert!(!is_native("LICENSE"));
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        use std::io::Write;

        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();

        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }

        writer.finish().unwrap();
    }

    #[tokio::test]
    async fn overrides_are_unpacked_with_their_structure() {
        let dir = std::env::temp_dir().join(format!("cast-zip-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let pack = dir.join("pack.mrpack");
        write_zip(
            &pack,
            &[
                ("modrinth.index.json", br#"{"name":"Pack"}"#),
                ("overrides/config/a.toml", b"a"),
                ("overrides/options.txt", b"b"),
                ("client-overrides/servers.dat", b"c"),
            ],
        );

        let target = dir.join("minecraft");
        let mut extracted = extract_dir(pack.clone(), "overrides".into(), target.clone())
            .await
            .unwrap();
        extracted.sort();

        assert_eq!(extracted, vec!["config/a.toml", "options.txt"]);
        assert_eq!(
            std::fs::read(target.join("config").join("a.toml")).unwrap(),
            b"a"
        );
        assert_eq!(std::fs::read(target.join("options.txt")).unwrap(), b"b");
        assert!(!target.join("servers.dat").exists());

        let index = read_entry(pack, "modrinth.index.json".into())
            .await
            .unwrap();
        assert_eq!(index, br#"{"name":"Pack"}"#);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_kept_file_is_not_overwritten_but_still_reported() {
        let dir = std::env::temp_dir().join(format!("cast-zip-{}", uuid::Uuid::new_v4()));
        let target = dir.join("minecraft");
        std::fs::create_dir_all(target.join("mods")).unwrap();
        std::fs::write(target.join("mods").join("a.jar"), b"mine").unwrap();

        let pack = dir.join("pack.zip");
        write_zip(
            &pack,
            &[
                ("overrides/mods/a.jar", b"pack"),
                ("overrides/mods/b.jar", b"pack"),
            ],
        );

        let mut extracted =
            extract_dir_with(pack, "overrides".into(), target.clone(), |key, path| {
                key == "mods/a.jar" && path.ends_with("mods/a.jar")
            })
            .await
            .unwrap();
        extracted.sort();

        assert_eq!(extracted, vec!["mods/a.jar", "mods/b.jar"]);
        assert_eq!(
            std::fs::read(target.join("mods").join("a.jar")).unwrap(),
            b"mine"
        );
        assert_eq!(
            std::fs::read(target.join("mods").join("b.jar")).unwrap(),
            b"pack"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_missing_override_directory_is_not_an_error() {
        let dir = std::env::temp_dir().join(format!("cast-zip-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let pack = dir.join("pack.mrpack");
        write_zip(&pack, &[("modrinth.index.json", b"{}")]);

        let extracted = extract_dir(pack.clone(), "client-overrides".into(), dir.join("mc"))
            .await
            .unwrap();
        assert!(extracted.is_empty());

        assert!(read_entry(pack, "нет.json".into()).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
