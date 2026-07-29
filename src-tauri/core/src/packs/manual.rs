use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};
use tokio::io::AsyncReadExt;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::safe_join;
use crate::packs::BlockedFile;

const MAX_CANDIDATE: u64 = 512 * 1024 * 1024;

const SEPARATORS: [char; 4] = ['-', '+', '.', '_'];

pub fn lax_equal(left: &str, right: &str) -> bool {
    normalize(left) == normalize(right)
}

fn normalize(name: &str) -> String {
    let replaced: String = name
        .to_lowercase()
        .chars()
        .map(|symbol| if SEPARATORS.contains(&symbol) { ' ' } else { symbol })
        .collect();

    replaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub async fn scan(dir: &Path, files: &mut [BlockedFile]) -> usize {
    if files.iter().all(BlockedFile::found) {
        return 0;
    }

    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return 0;
    };

    let mut candidates: Vec<PathBuf> = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        let Ok(metadata) = entry.metadata().await else { continue };

        if !metadata.is_file() || metadata.len() > MAX_CANDIDATE {
            continue;
        }

        candidates.push(path);
    }

    let mut found = 0;

    for file in files.iter_mut().filter(|file| !file.found()) {
        for candidate in &candidates {
            if matches(file, candidate).await {
                file.local_path = Some(candidate.display().to_string());
                found += 1;
                break;
            }
        }
    }

    found
}

async fn matches(file: &BlockedFile, candidate: &Path) -> bool {
    let Some(name) = candidate.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    if !lax_equal(name, &file.file_name) {
        return false;
    }

    let Some(expected) = &file.sha1 else {
        return true;
    };

    matches!(sha1(candidate).await, Some(actual) if actual.eq_ignore_ascii_case(expected))
}

pub async fn file_sha1(path: &Path) -> Option<String> {
    sha1(path).await
}

async fn sha1(path: &Path) -> Option<String> {
    let mut file = tokio::fs::File::open(path).await.ok()?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer).await.ok()?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Some(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

pub async fn place(minecraft_dir: &Path, file: &BlockedFile) -> CommandResult<String> {
    let source = file
        .local_path
        .as_ref()
        .ok_or_else(|| CommandError::fs(format!("Файл не найден на диске: {}", file.file_name)))?;

    let destination = safe_join(minecraft_dir, &file.target_path)?;

    if let Some(parent) = destination.parent() {
        crate::fs_util::ensure_dir(parent).await?;
    }

    tokio::fs::copy(source, &destination)
        .await
        .map_err(|e| CommandError::io("Не удалось скопировать скачанный файл", &destination, e))?;

    crate::fs_util::relative_key(&file.target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocked(file_name: &str, sha1: Option<&str>) -> BlockedFile {
        BlockedFile {
            file_name: file_name.to_string(),
            target_path: format!("mods/{file_name}"),
            website_url: "https://www.curseforge.com/minecraft/mc-mods/x".into(),
            sha1: sha1.map(str::to_string),
            local_path: None,
        }
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-manual-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, content: &[u8]) -> String {
        std::fs::write(dir.join(name), content).unwrap();

        let mut hasher = Sha1::new();
        hasher.update(content);
        hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn separators_and_case_do_not_matter_when_comparing_names() {
        assert!(lax_equal("sodium-extra-0.6.0.jar", "sodium_extra+0.6.0.jar"));
        assert!(lax_equal("EntityCulling-Fabric-1.10.5.jar", "entityculling-fabric-1.10.5.jar"));
        assert!(!lax_equal("sodium-0.5.jar", "sodium-0.6.jar"));
        assert!(!lax_equal("jei.jar", "rei.jar"));
    }

    #[tokio::test]
    async fn a_downloaded_file_is_matched_by_hash() {
        let dir = temp_dir();
        let hash = write(&dir, "entityculling-1.10.5.jar", "мод".as_bytes());

        let mut files = vec![blocked("entityculling-1.10.5.jar", Some(&hash))];

        assert_eq!(scan(&dir, &mut files).await, 1);
        assert!(files[0].found());
        assert!(files[0].local_path.as_ref().unwrap().ends_with("entityculling-1.10.5.jar"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn the_right_name_with_the_wrong_content_is_not_a_match() {
        let dir = temp_dir();
        write(&dir, "entityculling-1.10.5.jar", "не тот файл".as_bytes());

        let mut files = vec![blocked("entityculling-1.10.5.jar", Some(&"a".repeat(40)))];

        assert_eq!(scan(&dir, &mut files).await, 0);
        assert!(!files[0].found(), "подменённый файл в сборку не поедет");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn without_a_hash_the_name_is_enough() {
        let dir = temp_dir();
        write(&dir, "Sodium_Extra+0.6.0.jar", "что угодно".as_bytes());

        let mut files = vec![blocked("sodium-extra-0.6.0.jar", None)];

        assert_eq!(scan(&dir, &mut files).await, 1);
        assert!(files[0].found(), "имя отличается только разделителями");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn folders_can_be_added_one_after_another() {
        let first = temp_dir();
        let second = temp_dir();

        let a = write(&first, "a.jar", b"a");
        let b = write(&second, "b.jar", b"b");

        let mut files = vec![blocked("a.jar", Some(&a)), blocked("b.jar", Some(&b))];

        assert_eq!(scan(&first, &mut files).await, 1);
        assert!(files[0].found() && !files[1].found());

        assert_eq!(scan(&second, &mut files).await, 1);
        assert!(files.iter().all(BlockedFile::found));

        assert_eq!(scan(&first, &mut files).await, 0);

        std::fs::remove_dir_all(&first).ok();
        std::fs::remove_dir_all(&second).ok();
    }

    #[tokio::test]
    async fn a_missing_folder_is_not_an_error() {
        let mut files = vec![blocked("a.jar", None)];

        assert_eq!(scan(Path::new("/такой/папки/нет"), &mut files).await, 0);
        assert!(!files[0].found());
    }

    #[tokio::test]
    async fn subfolders_and_oversized_files_are_left_alone() {
        let dir = temp_dir();
        std::fs::create_dir_all(dir.join("вложенная")).unwrap();
        std::fs::write(dir.join("вложенная").join("a.jar"), b"a").unwrap();

        let mut files = vec![blocked("a.jar", None)];

        assert_eq!(scan(&dir, &mut files).await, 0, "в подпапки не заглядываем");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_found_file_lands_where_the_pack_expects_it() {
        let dir = temp_dir();
        let minecraft = dir.join("minecraft");
        std::fs::create_dir_all(&minecraft).unwrap();

        let hash = write(&dir, "jei.jar", "мод".as_bytes());
        let mut files = vec![blocked("jei.jar", Some(&hash))];

        scan(&dir, &mut files).await;

        let key = place(&minecraft, &files[0]).await.unwrap();

        assert_eq!(key, "mods/jei.jar");
        assert_eq!(std::fs::read(minecraft.join("mods").join("jei.jar")).unwrap(), b"\xd0\xbc\xd0\xbe\xd0\xb4");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_file_that_was_never_found_cannot_be_placed() {
        let dir = temp_dir();

        assert!(place(&dir, &blocked("jei.jar", None)).await.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_target_path_cannot_escape_the_instance() {
        let dir = temp_dir();
        let minecraft = dir.join("minecraft");
        std::fs::create_dir_all(&minecraft).unwrap();

        write(&dir, "evil.jar", b"evil");

        let mut file = blocked("evil.jar", None);
        file.target_path = "../../evil.jar".into();
        file.local_path = Some(dir.join("evil.jar").display().to_string());

        assert!(place(&minecraft, &file).await.is_err());
        assert!(!dir.parent().unwrap().join("evil.jar").exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
