use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{child_file, ensure_dir};

pub const MAX_SIZE: u64 = 8 * 1024 * 1024;

const EXTENSIONS: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
    ("gif", "image/gif"),
    ("bmp", "image/bmp"),
    ("svg", "image/svg+xml"),
    ("ico", "image/x-icon"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconFile {
    pub name: String,
    pub size: u64,
    pub modified: u64,
}

pub fn extensions() -> Vec<&'static str> {
    EXTENSIONS.iter().map(|(extension, _)| *extension).collect()
}

pub fn mime(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();

    EXTENSIONS
        .iter()
        .find(|(known, _)| *known == extension)
        .map(|(_, mime)| *mime)
}

pub async fn list(dir: &Path) -> CommandResult<Vec<IconFile>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог иконок", dir, e))?;

    let mut icons = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать каталог иконок", dir, e))?
    {
        let path = entry.path();

        if mime(&path).is_none() {
            continue;
        }

        let Ok(metadata) = entry.metadata().await else { continue };

        if !metadata.is_file() {
            continue;
        }

        icons.push(IconFile {
            name: entry.file_name().to_string_lossy().to_string(),
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|since| since.as_millis() as u64)
                .unwrap_or(0),
        });
    }

    icons.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));

    Ok(icons)
}

pub fn resolve(dir: &Path, name: &str) -> CommandResult<PathBuf> {
    let path = child_file(dir, name)?;

    if mime(&path).is_none() {
        return Err(CommandError::fs(format!("Это не картинка: {name}")));
    }

    Ok(path)
}

pub async fn import(dir: &Path, source: &Path) -> CommandResult<IconFile> {
    if mime(source).is_none() {
        return Err(CommandError::fs(format!(
            "Поддерживаются только картинки: {}",
            extensions().join(", ")
        )));
    }

    let metadata = tokio::fs::metadata(source)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать картинку", source, e))?;

    if !metadata.is_file() {
        return Err(CommandError::fs(format!("Это не файл: {}", source.display())));
    }

    if metadata.len() > MAX_SIZE {
        return Err(CommandError::fs(format!(
            "Картинка больше {} МБ",
            MAX_SIZE / 1024 / 1024
        )));
    }

    let bytes = tokio::fs::read(source)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать картинку", source, e))?;

    let name = source.file_name().map(|name| name.to_string_lossy().to_string());

    save(dir, &name.unwrap_or_else(|| "icon.png".into()), &bytes).await
}

pub async fn save(dir: &Path, name: &str, bytes: &[u8]) -> CommandResult<IconFile> {
    ensure_dir(dir).await?;

    let name = unique_name(dir, &sanitize_name(name));
    let path = resolve(dir, &name)?;

    crate::fs_util::write_atomic(&path, bytes).await?;

    Ok(IconFile {
        name,
        size: bytes.len() as u64,
        modified: now_millis(),
    })
}

pub async fn save_once(dir: &Path, name: &str, bytes: &[u8]) -> CommandResult<IconFile> {
    let name = sanitize_name(name);
    let path = resolve(dir, &name)?;

    if let Ok(metadata) = tokio::fs::metadata(&path).await {
        if metadata.is_file() {
            return Ok(IconFile {
                name,
                size: metadata.len(),
                modified: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|since| since.as_millis() as u64)
                    .unwrap_or(0),
            });
        }
    }

    ensure_dir(dir).await?;
    crate::fs_util::write_atomic(&path, bytes).await?;

    Ok(IconFile {
        name,
        size: bytes.len() as u64,
        modified: now_millis(),
    })
}

pub async fn data_url(path: &Path) -> CommandResult<String> {
    let mime = mime(path).ok_or_else(|| CommandError::fs(format!("Это не картинка: {}", path.display())))?;

    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| CommandError::io("Не удалось прочитать иконку", path, e))?;

    Ok(to_data_url(mime, &bytes))
}

pub fn to_data_url(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

pub async fn remove(dir: &Path, name: &str) -> CommandResult<()> {
    let path = resolve(dir, name)?;

    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| CommandError::io("Не удалось удалить иконку", &path, e))
}

fn sanitize_name(name: &str) -> String {
    let name = Path::new(name)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    let extension = Path::new(&name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .filter(|extension| EXTENSIONS.iter().any(|(known, _)| known == extension))
        .unwrap_or_else(|| "png".to_string());

    let stem: String = Path::new(&name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default()
        .chars()
        .map(|symbol| match symbol {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => symbol,
            ' ' => '-',
            symbol if symbol.is_alphanumeric() => symbol,
            _ => '-',
        })
        .collect();

    let stem: String = stem.trim_matches(['-', '.']).chars().take(64).collect();
    let stem = if stem.is_empty() { "icon".to_string() } else { stem };

    format!("{stem}.{extension}")
}

fn unique_name(dir: &Path, name: &str) -> String {
    let path = Path::new(name);

    let stem = path.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_default();
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_else(|| "png".into());

    let mut candidate = name.to_string();
    let mut index = 1;

    while dir.join(&candidate).exists() {
        candidate = format!("{stem}-{index}.{extension}");
        index += 1;
    }

    candidate
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

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-icons-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn only_images_are_treated_as_icons() {
        assert_eq!(mime(Path::new("a.PNG")), Some("image/png"));
        assert_eq!(mime(Path::new("a.webp")), Some("image/webp"));
        assert_eq!(mime(Path::new("a.exe")), None);
        assert_eq!(mime(Path::new("noext")), None);
    }

    #[test]
    fn resolve_rejects_escapes_and_non_images() {
        let dir = Path::new("/icons");

        assert!(resolve(dir, "../config.json").is_err());
        assert!(resolve(dir, "virus.exe").is_err());
        assert_eq!(resolve(dir, "grass.png").unwrap(), dir.join("grass.png"));
    }

    #[test]
    fn names_are_reduced_to_a_safe_file_name() {
        assert_eq!(sanitize_name("Моя иконка.PNG"), "Моя-иконка.png");
        assert_eq!(sanitize_name("../../etc/passwd"), "passwd.png");
        assert_eq!(sanitize_name("shot.jpeg"), "shot.jpeg");
        assert_eq!(sanitize_name("..."), "icon.png");
        assert_eq!(sanitize_name(".png"), "png.png");
        assert_eq!(sanitize_name("archive.zip"), "archive.png");
    }

    #[tokio::test]
    async fn importing_twice_keeps_both_files() {
        let dir = temp_dir();
        let source = dir.join("source.png");
        std::fs::write(&source, b"first").unwrap();

        let library = dir.join("library");
        let one = import(&library, &source).await.unwrap();

        std::fs::write(&source, b"second").unwrap();
        let two = import(&library, &source).await.unwrap();

        assert_eq!(one.name, "source.png");
        assert_eq!(two.name, "source-1.png");
        assert_eq!(list(&library).await.unwrap().len(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn importing_a_non_image_is_rejected() {
        let dir = temp_dir();
        let source = dir.join("mod.jar");
        std::fs::write(&source, b"nope").unwrap();

        assert!(import(&dir.join("library"), &source).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn saving_the_same_catalog_icon_twice_reuses_the_file() {
        let dir = temp_dir();

        let one = save_once(&dir, "mc-grass_block.webp", b"pixels").await.unwrap();
        let two = save_once(&dir, "mc-grass_block.webp", b"pixels").await.unwrap();

        assert_eq!(one.name, two.name);
        assert_eq!(list(&dir).await.unwrap().len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn data_url_carries_the_mime_type() {
        let dir = temp_dir();
        let file = dir.join("icon.png");
        std::fs::write(&file, b"pixels").unwrap();

        let url = data_url(&file).await.unwrap();

        assert!(url.starts_with("data:image/png;base64,"));
        assert!(url.ends_with(&STANDARD.encode(b"pixels")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn list_skips_foreign_files_and_missing_directories() {
        let dir = temp_dir();
        std::fs::write(dir.join("icon.png"), b"a").unwrap();
        std::fs::write(dir.join("notes.txt"), b"b").unwrap();
        std::fs::create_dir(dir.join("nested.png")).unwrap();

        let icons = list(&dir).await.unwrap();

        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].name, "icon.png");
        assert!(list(&dir.join("нет")).await.unwrap().is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
