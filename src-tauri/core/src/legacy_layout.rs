//! # PERMANENTLY DEPRECATED
//!
//! По 1.4.0 включительно `instances/`, `icons/` и `accounts.json` всегда лежали в
//! каталоге конфигурации, даже если в настройках был задан свой путь: настройка
//! "Файлы лаунчера" двигала только библиотеки, ассеты, кэш, рантаймы и логи.
//! Теперь туда уезжает вообще всё, что относится к игре, так что у тех, кто успел
//! сменить каталог, сборки с аккаунтами остались брошенными в `%APPDATA%`.
//! Этот модуль переносит их один раз при старте.
//!
//! # Как выпилить
//!
//! Когда обновятся все живые установки (ориентир — пара релизов после 1.5.0):
//!
//! 1. удалить этот файл;
//! 2. убрать `pub mod legacy_layout;` из `core/src/lib.rs`;
//! 3. убрать единственный вызов `legacy_layout::migrate` в `AppState::initialize`
//!    вместе с логированием отчёта.
//!
//! Больше модуль ниоткуда не используется и ничего за собой не оставляет: ни
//! флагов в конфиге, ни маркеров на диске. Повторный запуск после переезда
//! просто ничего не находит и молча заканчивается.

use std::path::Path;

use crate::error::{CommandError, CommandResult};
use crate::fs_util::{ensure_dir, merge_dir, remove_dir_if_exists, remove_file_if_exists};
use crate::paths::LauncherPaths;

/// Всё, что раньше жило в каталоге конфигурации, а теперь относится к каталогу игры.
const LEGACY_ENTRIES: [&str; 3] = ["instances", "icons", "accounts.json"];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub moved: Vec<String>,
    pub failed: Vec<(String, String)>,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.moved.is_empty() && self.failed.is_empty()
    }
}

/// Переносит старую раскладку в каталог лаунчера. Ошибка одного элемента не
/// останавливает остальные и никогда не роняет запуск: в худшем случае файлы
/// останутся на старом месте, и лаунчер просто увидит пустой список сборок.
///
/// Вызывать строго до того, как кто-то создаст `instances/` в новом каталоге.
pub async fn migrate(paths: &LauncherPaths) -> Report {
    let mut report = Report::default();

    if paths.root() == paths.config_root() {
        return report;
    }

    for name in LEGACY_ENTRIES {
        let from = paths.config_root().join(name);
        let to = paths.root().join(name);

        match relocate(&from, &to).await {
            Ok(true) => report.moved.push(name.to_string()),
            Ok(false) => {}
            Err(error) => report.failed.push((name.to_string(), error.to_string())),
        }
    }

    report
}

/// `Ok(false)` — переносить было нечего.
async fn relocate(from: &Path, to: &Path) -> CommandResult<bool> {
    if !from.exists() {
        return Ok(false);
    }

    if from.is_file() {
        // Аккаунты в новом каталоге уже могли появиться — они свежее старых.
        if to.exists() {
            return Ok(false);
        }

        ensure_parent(to).await?;

        if rename(from, to).await {
            return Ok(true);
        }

        tokio::fs::copy(from, to)
            .await
            .map_err(|e| CommandError::io("Не удалось перенести файл", from, e))?;

        remove_file_if_exists(from).await;

        return Ok(true);
    }

    ensure_parent(to).await?;

    if !to.exists() && rename(from, to).await {
        return Ok(true);
    }

    // Между дисками переименования нет, а именно так и стоит настройка у тех,
    // ради кого написан этот модуль. `merge_dir` копирует и никогда не трогает
    // то, что уже лежит в новом каталоге, поэтому повторный запуск безопасен.
    merge_dir(from, to).await?;
    remove_dir_if_exists(from).await;

    Ok(true)
}

async fn ensure_parent(path: &Path) -> CommandResult<()> {
    match path.parent() {
        Some(parent) => ensure_dir(parent).await,
        None => Ok(()),
    }
}

/// Внутри одного диска это мгновенно, между дисками — всегда ошибка.
async fn rename(from: &Path, to: &Path) -> bool {
    tokio::fs::rename(from, to).await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("cast-legacy-{}", uuid::Uuid::new_v4()))
    }

    async fn write(path: &Path, text: &str) {
        ensure_parent(path).await.unwrap();
        tokio::fs::write(path, text).await.unwrap();
    }

    async fn read(path: &Path) -> String {
        tokio::fs::read_to_string(path).await.unwrap()
    }

    /// Каталог конфигурации со старой раскладкой и отдельный каталог лаунчера.
    async fn old_layout() -> (PathBuf, PathBuf) {
        let base = temp_dir();
        let config_root = base.join("cfg");
        let root = base.join("data");

        write(&config_root.join("instances/abc/instance.json"), "{}").await;
        write(&config_root.join("instances/abc/minecraft/mods/mod.jar"), "jar").await;
        write(&config_root.join("icons/pack.png"), "png").await;
        write(&config_root.join("accounts.json"), "старые аккаунты").await;
        write(&config_root.join("config.json"), "конфиг").await;

        (config_root, root)
    }

    #[tokio::test]
    async fn the_default_launcher_dir_has_nothing_to_migrate() {
        let (config_root, _) = old_layout().await;
        let paths = LauncherPaths::new(config_root.clone(), None);

        assert!(migrate(&paths).await.is_empty());
        assert!(config_root.join("instances/abc/instance.json").exists());
    }

    #[tokio::test]
    async fn the_old_layout_follows_the_launcher_dir() {
        let (config_root, root) = old_layout().await;
        let paths = LauncherPaths::new(config_root.clone(), Some(&root.display().to_string()));

        let report = migrate(&paths).await;

        assert_eq!(report.moved, ["instances", "icons", "accounts.json"]);
        assert!(report.failed.is_empty());

        assert_eq!(read(&root.join("instances/abc/minecraft/mods/mod.jar")).await, "jar");
        assert_eq!(read(&root.join("icons/pack.png")).await, "png");
        assert_eq!(read(&root.join("accounts.json")).await, "старые аккаунты");

        assert!(!config_root.join("instances").exists());
        assert!(!config_root.join("icons").exists());
        assert!(!config_root.join("accounts.json").exists());
    }

    #[tokio::test]
    async fn the_config_itself_stays_where_it_is() {
        let (config_root, root) = old_layout().await;
        let paths = LauncherPaths::new(config_root.clone(), Some(&root.display().to_string()));

        migrate(&paths).await;

        assert_eq!(read(&config_root.join("config.json")).await, "конфиг");
        assert!(!root.join("config.json").exists());
    }

    #[tokio::test]
    async fn what_is_already_in_the_new_place_wins() {
        let (config_root, root) = old_layout().await;
        let paths = LauncherPaths::new(config_root.clone(), Some(&root.display().to_string()));

        write(&root.join("accounts.json"), "новые аккаунты").await;
        write(&root.join("instances/abc/instance.json"), "новая сборка").await;

        let report = migrate(&paths).await;

        assert_eq!(report.moved, ["instances", "icons"]);
        assert_eq!(read(&root.join("accounts.json")).await, "новые аккаунты");
        assert_eq!(read(&root.join("instances/abc/instance.json")).await, "новая сборка");
        // Всё остальное из старой сборки при этом всё равно доезжает.
        assert_eq!(read(&root.join("instances/abc/minecraft/mods/mod.jar")).await, "jar");
    }

    #[tokio::test]
    async fn a_second_launch_finds_nothing_left_to_do() {
        let (config_root, root) = old_layout().await;
        let paths = LauncherPaths::new(config_root, Some(&root.display().to_string()));

        migrate(&paths).await;

        assert!(migrate(&paths).await.is_empty());
    }
}
