use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cast_core::archive;
use cast_core::error::{CommandError, CommandResult};
use cast_core::fs_util::child_file;
use cast_core::install::pack_files::{self, PackFiles};
use cast_core::instance::{Instance, PackProvider, PackSource};
use cast_core::net::download::{DownloadOptions, DownloadTask};
use cast_core::packs::{BlockedFile, ResolvedPack};
use cast_core::paths::LauncherPaths;

use super::{blocked, download_reporter, job_id, ProgressReporter};
use crate::state::AppState;

pub struct Modpack {
    archive: PathBuf,
    resolved: ResolvedPack,
}

impl Modpack {
    pub fn resolved(&self) -> &ResolvedPack {
        &self.resolved
    }
}

pub async fn prepare(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    pack: &PackSource,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<Modpack> {
    reporter.set_message(format!("Загрузка модпака {}", pack.version_number));

    let archive_path = archive_path(paths, pack)?;

    match pack.file_url.trim().is_empty() {
        true => fetch_archive_by_hand(state, instance, pack, &archive_path, reporter).await?,
        false => {
            let task = DownloadTask::verified(
                pack.file_url.clone(),
                archive_path.clone(),
                pack.file_size,
                pack.file_sha1.clone(),
            );

            state
                .downloads
                .run(
                    job_id(&instance.id, "modpack-archive"),
                    vec![task],
                    DownloadOptions::default(),
                    Some(download_reporter(reporter)),
                )
                .await?;
        }
    }

    let minecraft = paths.instance(&instance.id).minecraft();

    let resolved = match pack.provider {
        PackProvider::Modrinth => {
            use cast_core::modrinth::pack::{PackIndex, INDEX_ENTRY};

            let manifest = archive::read_entry(archive_path.clone(), INDEX_ENTRY.to_string()).await?;

            PackIndex::parse(&manifest)?.resolve(&minecraft)?
        }
        PackProvider::CurseForge => {
            use cast_core::curseforge::pack::{self, Manifest, MANIFEST_ENTRY};

            let manifest = archive::read_entry(archive_path.clone(), MANIFEST_ENTRY.to_string()).await?;
            let manifest = Manifest::parse(&manifest)?;

            reporter.begin_phase("modpack-resolve", "Список файлов пака");
            reporter.set_message(format!("Проверка {} файлов пака", manifest.files.len()));

            let resolved = pack::resolve(&manifest, &minecraft).await?;

            reporter.set_fraction(1.0);
            resolved
        }
    };

    if !resolved.blocked.is_empty() {
        reporter.set_blocked(resolved.blocked.clone());
    }

    Ok(Modpack {
        archive: archive_path,
        resolved,
    })
}

async fn fetch_archive_by_hand(
    state: &Arc<AppState>,
    instance: &Instance,
    pack: &PackSource,
    archive_path: &Path,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    if blocked::already_there(archive_path, pack.file_sha1.as_deref()).await {
        return Ok(());
    }

    let file_name = match pack.file_name.trim().is_empty() {
        true => format!("{}.zip", pack.version_number),
        false => pack.file_name.clone(),
    };

    let website_url = match pack.provider {
        PackProvider::CurseForge => cast_core::curseforge::download_page(&pack.project_id, &pack.version_id)
            .await
            .unwrap_or_default(),
        PackProvider::Modrinth => String::new(),
    };

    let wanted = BlockedFile {
        file_name,
        target_path: pack.file_name.clone(),
        website_url,
        sha1: pack.file_sha1.clone(),
        local_path: None,
    };

    let found = state
        .blocked
        .wait(&instance.id, vec![wanted], reporter)
        .await;

    super::check_cancelled(reporter)?;

    let Some(source) = found.first().and_then(|file| file.local_path.clone()) else {
        return Err(CommandError::download(format!(
            "Без архива «{}» установить сборку нельзя: автор запретил скачивание через сторонние лаунчеры, \
             поэтому его нужно скачать со страницы пака вручную",
            pack.version_number
        )));
    };

    if let Some(parent) = archive_path.parent() {
        cast_core::fs_util::ensure_dir(parent).await?;
    }

    tokio::fs::copy(&source, archive_path)
        .await
        .map_err(|e| CommandError::io("Не удалось скопировать архив модпака", archive_path, e))?;

    Ok(())
}

pub async fn sync_instance(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    resolved: &ResolvedPack,
) -> CommandResult<Instance> {
    let loader = resolved.loader;
    let loader_version = resolved.loader_version.clone();
    let minecraft_version = resolved.minecraft_version.clone();

    if instance.installed && loader != instance.loader {
        return Err(CommandError::manifest(format!(
            "Модпак собран под {}, а сборка установлена под {}",
            loader.label(),
            instance.loader.label()
        )));
    }

    let unchanged = instance.loader == loader
        && instance.minecraft_version == minecraft_version
        && instance.loader_version == loader_version;

    if unchanged {
        return Ok(instance.clone());
    }

    state
        .instances
        .update(paths, &instance.id, move |current| {
            current.loader = loader;
            current.minecraft_version = minecraft_version;
            current.loader_version = loader_version;
        })
        .await
}

pub async fn apply(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    pack: &PackSource,
    modpack: &Modpack,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    reporter.begin_phase("modpack", "Файлы модпака");

    let instance_paths = paths.instance(&instance.id);
    let minecraft = instance_paths.minecraft();

    let previous = PackFiles::load(&instance_paths.pack_files()).await;

    let resolved = &modpack.resolved;
    let mut owned: BTreeSet<String> = resolved.paths.iter().cloned().collect();

    let blocked = match resolved.blocked.is_empty() {
        true => Vec::new(),
        false => {
            let found = state
                .blocked
                .wait(&instance.id, resolved.blocked.clone(), reporter)
                .await;

            super::check_cancelled(reporter)?;

            reporter.begin_phase("modpack", "Файлы модпака");
            reporter.set_message("Перенос скачанных вручную файлов");

            owned.extend(blocked::place_found(&minecraft, &found).await);

            found
        }
    };

    if !resolved.tasks.is_empty() {
        state
            .downloads
            .run(
                job_id(&instance.id, "modpack"),
                resolved.tasks.clone(),
                DownloadOptions::default(),
                Some(download_reporter(reporter)),
            )
            .await?;
    }

    reporter.set_message("Распаковка модпака");

    for prefix in &resolved.overrides {
        let extracted =
            archive::extract_dir(modpack.archive.clone(), prefix.clone(), minecraft.clone()).await?;

        owned.extend(extracted);
    }

    let stale = previous.stale(&owned);

    if !stale.is_empty() {
        reporter.set_message(format!("Удаление файлов прошлой версии: {}", stale.len()));
        pack_files::remove(&minecraft, &stale).await;
    }

    PackFiles::new(pack.version_id.clone(), owned)
        .save(&instance_paths.pack_files())
        .await?;

    let missing: Vec<_> = blocked.into_iter().filter(|file| !file.found()).collect();

    pack_files::save_blocked(&instance_paths.pack_blocked(), &missing).await?;
    reporter.set_blocked(missing);

    reporter.set_fraction(1.0);

    Ok(())
}

fn archive_path(paths: &LauncherPaths, pack: &PackSource) -> CommandResult<PathBuf> {
    let key: String = pack
        .version_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(32)
        .collect();

    if key.is_empty() {
        return Err(CommandError::manifest(format!(
            "Некорректная версия модпака: {}",
            pack.version_id
        )));
    }

    let name = format!(
        "{}-{key}.{}",
        pack.provider.key(),
        pack.provider.archive_extension()
    );

    child_file(&paths.cache().join("modpacks"), &name)
}
