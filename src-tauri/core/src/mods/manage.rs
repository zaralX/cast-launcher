use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{ensure_dir, relative_key};

use super::index::ModsIndex;
use super::{is_mod_file, ModsScan, DISABLED_SUFFIX, FOLDER};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub added: Vec<String>,
    pub replaced: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

pub async fn set_enabled(scan: &ModsScan, path: &str, enabled: bool) -> CommandResult<()> {
    let (name, from) = target(scan, path)?;
    let renamed = switched(&name, enabled);

    if renamed == name {
        return Ok(());
    }

    let to = scan.dir.join(&renamed);

    if to.exists() {
        return Err(CommandError::fs(format!(
            "В папке модов уже есть «{renamed}» - переименуйте или удалите его"
        )));
    }

    tokio::fs::rename(&from, &to)
        .await
        .map_err(|e| CommandError::io("Не удалось переключить мод", &from, e))?;

    let mut index = ModsIndex::load(&scan.index_file).await;

    if index.rename(path, &key(&renamed)) {
        let _ = index.save(&scan.index_file).await;
    }

    Ok(())
}

pub async fn remove(scan: &ModsScan, paths: &[String]) -> CommandResult<usize> {
    let mut index = ModsIndex::load(&scan.index_file).await;

    let mut removed = 0;
    let mut forgotten = false;
    let mut failure = None;

    for path in paths {
        let (_, file) = target(scan, path)?;

        let result = match file.is_dir() {
            true => tokio::fs::remove_dir_all(&file).await,
            false => tokio::fs::remove_file(&file).await,
        };

        match result {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                failure = Some(CommandError::io("Не удалось удалить мод", &file, error));
                continue;
            }
        }

        forgotten |= index.forget(path);
    }

    if forgotten {
        let _ = index.save(&scan.index_file).await;
    }

    match failure {
        Some(error) => Err(error),
        None => Ok(removed),
    }
}

pub async fn install(scan: &ModsScan, sources: &[PathBuf]) -> CommandResult<Installed> {
    let mut report = Installed::default();

    if sources.is_empty() {
        return Ok(report);
    }

    ensure_dir(&scan.dir).await?;

    let mut index = ModsIndex::load(&scan.index_file).await;
    let mut forgotten = false;

    for source in sources {
        let name = source
            .file_name()
            .map(|name| crate::curseforge::pack::sanitize(&name.to_string_lossy()))
            .unwrap_or_default();

        if name.is_empty() || !source.is_file() || !is_mod_file(&name) {
            report.skipped.push(display_name(source));
            continue;
        }

        let destination = scan.dir.join(&name);

        if same_file(source, &destination) {
            report.skipped.push(name);
            continue;
        }

        let switched_off = scan.dir.join(format!("{name}{DISABLED_SUFFIX}"));
        let existed = destination.exists() || switched_off.is_file();

        if let Err(error) = tokio::fs::copy(source, &destination).await {
            eprintln!("Не удалось скопировать {}: {error}", source.display());
            report.failed.push(name);
            continue;
        }

        let _ = tokio::fs::remove_file(&switched_off).await;

        forgotten |= index.forget(&key(&name));
        forgotten |= index.forget(&key(&format!("{name}{DISABLED_SUFFIX}")));

        match existed {
            true => report.replaced.push(name),
            false => report.added.push(name),
        }
    }

    if forgotten {
        let _ = index.save(&scan.index_file).await;
    }

    Ok(report)
}

fn switched(name: &str, enabled: bool) -> String {
    match enabled {
        true => name.trim_end_matches(DISABLED_SUFFIX).to_string(),
        false => match name.ends_with(DISABLED_SUFFIX) {
            true => name.to_string(),
            false => format!("{name}{DISABLED_SUFFIX}"),
        },
    }
}

fn key(file_name: &str) -> String {
    format!("{FOLDER}/{file_name}")
}

fn target(scan: &ModsScan, path: &str) -> CommandResult<(String, PathBuf)> {
    let outside = || CommandError::fs(format!("Мод «{path}» лежит не в папке модов"));

    let key = relative_key(path).map_err(|_| outside())?;
    let (folder, name) = key.split_once('/').ok_or_else(outside)?;

    if folder != FOLDER || name.is_empty() || name.contains('/') {
        return Err(outside());
    }

    Ok((name.to_string(), scan.dir.join(name)))
}

fn same_file(source: &Path, destination: &Path) -> bool {
    match (source.canonicalize(), destination.canonicalize()) {
        (Ok(from), Ok(to)) => from == to,
        _ => false,
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::{list, ModDetails};

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-manage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn scan_in(root: &Path) -> ModsScan {
        let dir = root.join("minecraft").join(FOLDER);
        std::fs::create_dir_all(&dir).unwrap();

        ModsScan {
            dir,
            index_file: root.join("mods-index.json"),
            icons: root.join("mod-icons"),
            loader: None,
            managed: Default::default(),
        }
    }

    fn put(scan: &ModsScan, name: &str) {
        std::fs::write(scan.dir.join(name), "содержимое мода".as_bytes()).unwrap();
    }

    async fn remembered(scan: &ModsScan, path: &str) -> bool {
        ModsIndex::load(&scan.index_file)
            .await
            .entries
            .contains_key(path)
    }

    async fn remember(scan: &ModsScan, path: &str) {
        let mut index = ModsIndex::load(&scan.index_file).await;
        index.remember(path, 1, 1, &ModDetails::default());
        index.save(&scan.index_file).await.unwrap();
    }

    #[tokio::test]
    async fn a_mod_switches_off_and_back_on() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar");

        set_enabled(&scan, "mods/jei.jar", false).await.unwrap();

        assert!(scan.dir.join("jei.jar.disabled").is_file());
        assert!(!scan.dir.join("jei.jar").exists());

        set_enabled(&scan, "mods/jei.jar.disabled", true)
            .await
            .unwrap();

        assert!(scan.dir.join("jei.jar").is_file());
        assert!(!scan.dir.join("jei.jar.disabled").exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn switching_keeps_the_parsed_metadata() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar");
        remember(&scan, "mods/jei.jar").await;

        set_enabled(&scan, "mods/jei.jar", false).await.unwrap();

        assert!(!remembered(&scan, "mods/jei.jar").await);
        assert!(
            remembered(&scan, "mods/jei.jar.disabled").await,
            "jar не перечитывается"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn switching_into_an_occupied_name_is_refused() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar");
        put(&scan, "jei.jar.disabled");

        let error = set_enabled(&scan, "mods/jei.jar", false).await.unwrap_err();

        assert!(
            error.message.contains("jei.jar.disabled"),
            "{}",
            error.message
        );
        assert!(scan.dir.join("jei.jar").is_file(), "файл на месте");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn switching_to_the_state_it_already_has_does_nothing() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar");

        set_enabled(&scan, "mods/jei.jar", true).await.unwrap();

        assert!(scan.dir.join("jei.jar").is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn nothing_outside_the_mods_folder_can_be_touched() {
        let root = temp_dir();
        let scan = scan_in(&root);

        for path in [
            "../instance.json",
            "mods/../../secret",
            "config/a.toml",
            "jei.jar",
            "mods/nested/a.jar",
        ] {
            assert!(set_enabled(&scan, path, false).await.is_err(), "{path}");
            assert!(remove(&scan, &[path.to_string()]).await.is_err(), "{path}");
        }

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn deleting_takes_the_file_and_its_cache_entry() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar");
        put(&scan, "sodium.jar");
        remember(&scan, "mods/jei.jar").await;

        let removed = remove(&scan, &["mods/jei.jar".to_string()]).await.unwrap();

        assert_eq!(removed, 1);
        assert!(!scan.dir.join("jei.jar").exists());
        assert!(scan.dir.join("sodium.jar").is_file());
        assert!(!remembered(&scan, "mods/jei.jar").await);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn deleting_an_unpacked_mod_takes_the_whole_folder() {
        let root = temp_dir();
        let scan = scan_in(&root);
        std::fs::create_dir_all(scan.dir.join("unpacked")).unwrap();
        std::fs::write(scan.dir.join("unpacked").join("fabric.mod.json"), b"{}").unwrap();

        remove(&scan, &["mods/unpacked".to_string()]).await.unwrap();

        assert!(!scan.dir.join("unpacked").exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn deleting_what_is_already_gone_is_not_an_error() {
        let root = temp_dir();
        let scan = scan_in(&root);

        assert_eq!(
            remove(&scan, &["mods/ghost.jar".to_string()])
                .await
                .unwrap(),
            0
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn adding_copies_mods_and_reports_the_rest() {
        let root = temp_dir();
        let scan = scan_in(&root);

        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("sodium.jar"), b"new").unwrap();
        std::fs::write(source.join("notes.txt"), b"nope").unwrap();
        std::fs::write(source.join("old.jar"), b"replacement").unwrap();
        put(&scan, "old.jar");

        let report = install(
            &scan,
            &[
                source.join("sodium.jar"),
                source.join("notes.txt"),
                source.join("old.jar"),
                source.join("ghost.jar"),
            ],
        )
        .await
        .unwrap();

        assert_eq!(report.added, vec!["sodium.jar"]);
        assert_eq!(report.replaced, vec!["old.jar"]);
        assert_eq!(report.skipped, vec!["notes.txt", "ghost.jar"]);
        assert_eq!(
            std::fs::read(scan.dir.join("old.jar")).unwrap(),
            b"replacement"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_file_that_could_not_be_copied_does_not_stop_the_rest() {
        let root = temp_dir();
        let scan = scan_in(&root);

        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("sodium.jar"), b"new").unwrap();

        // Каталог вместо файла: скопировать его нельзя, но сосед должен доехать.
        std::fs::create_dir_all(source.join("broken.jar")).unwrap();

        let report = install(
            &scan,
            &[source.join("broken.jar"), source.join("sodium.jar")],
        )
        .await
        .unwrap();

        assert_eq!(report.added, vec!["sodium.jar"]);
        assert!(scan.dir.join("sodium.jar").is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_failed_copy_keeps_the_switched_off_copy() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar.disabled");

        let missing = root.join("source").join("jei.jar");

        let report = install(&scan, &[missing]).await.unwrap();

        assert_eq!(report.skipped, vec!["jei.jar"]);
        assert!(
            scan.dir.join("jei.jar.disabled").is_file(),
            "выключенная копия на месте"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn adding_a_mod_that_is_lying_there_switched_off_replaces_it() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar.disabled");

        let source = root.join("jei.jar");
        std::fs::write(&source, b"fresh").unwrap();

        let report = install(&scan, &[source]).await.unwrap();

        assert_eq!(report.replaced, vec!["jei.jar"]);
        assert!(
            !scan.dir.join("jei.jar.disabled").exists(),
            "выключенной копии не осталось"
        );
        assert!(scan.dir.join("jei.jar").is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn adding_a_file_that_is_already_in_the_folder_does_not_wipe_it() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar");

        let report = install(&scan, &[scan.dir.join("jei.jar")]).await.unwrap();

        assert_eq!(report.skipped, vec!["jei.jar"]);
        assert!(scan.dir.join("jei.jar").is_file());

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_list_follows_what_was_done_to_the_folder() {
        let root = temp_dir();
        let scan = scan_in(&root);
        put(&scan, "jei.jar");

        set_enabled(&scan, "mods/jei.jar", false).await.unwrap();

        let mods = list(&scan, false).await.unwrap();

        assert_eq!(mods.len(), 1);
        assert!(!mods[0].enabled);
        assert_eq!(mods[0].path, "mods/jei.jar.disabled");
        assert_eq!(mods[0].display_name(), "jei");

        std::fs::remove_dir_all(&root).ok();
    }
}
