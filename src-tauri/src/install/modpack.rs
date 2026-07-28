use std::path::PathBuf;
use std::sync::Arc;

use cast_core::archive;
use cast_core::error::{CommandError, CommandResult};
use cast_core::fs_util::child_file;
use cast_core::instance::{Instance, PackSource};
use cast_core::modrinth::pack::{PackIndex, INDEX_ENTRY, OVERRIDES};
use cast_core::net::download::{DownloadOptions, DownloadTask};
use cast_core::paths::LauncherPaths;

use super::{download_reporter, job_id, ProgressReporter};
use crate::state::AppState;

pub struct Modpack {
    archive: PathBuf,
    index: PackIndex,
}

impl Modpack {
    pub fn index(&self) -> &PackIndex {
        &self.index
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

    let manifest = archive::read_entry(archive_path.clone(), INDEX_ENTRY.to_string()).await?;

    Ok(Modpack {
        archive: archive_path,
        index: PackIndex::parse(&manifest)?,
    })
}

pub async fn sync_instance(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    index: &PackIndex,
) -> CommandResult<Instance> {
    let (loader, loader_version) = index.loader()?;
    let minecraft_version = index.minecraft_version()?.to_string();

    if loader != instance.loader {
        return Err(CommandError::manifest(format!(
            "Модпак собран под {}, а сборка создана под {}",
            loader.label(),
            instance.loader.label()
        )));
    }

    if instance.minecraft_version == minecraft_version && instance.loader_version == loader_version {
        return Ok(instance.clone());
    }

    state
        .instances
        .update(paths, &instance.id, move |current| {
            current.minecraft_version = minecraft_version;
            current.loader_version = loader_version;
        })
        .await
}

pub async fn apply(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    modpack: &Modpack,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    reporter.begin_phase("modpack", "Файлы модпака");

    let minecraft = paths.instance(&instance.id).minecraft();
    let tasks = modpack.index.client_tasks(&minecraft)?;

    if !tasks.is_empty() {
        state
            .downloads
            .run(
                job_id(&instance.id, "modpack"),
                tasks,
                DownloadOptions::default(),
                Some(download_reporter(reporter)),
            )
            .await?;
    }

    reporter.set_message("Распаковка модпака");

    for prefix in OVERRIDES {
        archive::extract_dir(modpack.archive.clone(), (*prefix).to_string(), minecraft.clone()).await?;
    }

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

    child_file(&paths.cache().join("modpacks"), &format!("{key}.mrpack"))
}
