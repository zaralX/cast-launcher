use std::path::PathBuf;
use std::sync::Arc;

use cast_core::castpack::{self, Manifest, Overlay};
use cast_core::error::CommandResult;
use cast_core::instance::{CastPackSource, Instance, PackSource};
use cast_core::packs::ResolvedPack;
use cast_core::paths::LauncherPaths;

use super::modpack;
use super::ProgressReporter;
use crate::state::AppState;

pub struct CastPack {
    pub manifest: Manifest,
    resolved: ResolvedPack,
    archive: Option<PathBuf>,
    base: Option<PackSource>,
}

impl CastPack {
    pub fn resolved(&self) -> &ResolvedPack {
        &self.resolved
    }

    pub fn resolved_mut(&mut self) -> &mut ResolvedPack {
        &mut self.resolved
    }
}

pub async fn prepare(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    source: &CastPackSource,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<CastPack> {
    reporter.begin_phase("castpack-manifest", "Манифест сборки");

    let saved = paths.instance(&instance.id).castpack_manifest();
    let manifest = read_manifest(&source.manifest_url, &saved).await?;

    reporter.set_fraction(1.0);
    super::check_cancelled(reporter)?;

    let minecraft = paths.instance(&instance.id).minecraft();

    let (base, base_pack, archive) = match crate::castpack::base_pack(&manifest).await? {
        Some((pack, _, _)) => {
            let prepared = modpack::prepare(state, paths, instance, &pack, reporter).await?;

            super::check_cancelled(reporter)?;

            let (archive, resolved) = prepared.into_parts();

            (Some(resolved), Some(pack), Some(archive))
        }
        None => (None, None, None),
    };

    reporter.begin_phase("castpack-mods", "Список модов сборки");
    reporter.set_message(format!("Проверка {} модов", manifest.mods.len()));

    let mods = castpack::mods::resolve(&manifest.catalog_mods()?, &minecraft).await?;

    reporter.set_fraction(1.0);
    super::check_cancelled(reporter)?;

    let mut files = mods.files;
    files.extend(manifest.direct_mods(&minecraft)?);
    files.extend(manifest.owned_files(&minecraft)?);

    let overlay = Overlay {
        minecraft_version: manifest.minecraft_version().map(str::to_string),
        loader: manifest.loader(),
        files,
        seed: manifest.seed_files(&minecraft)?,
        delete: manifest.delete_keys()?,
        blocked: mods.blocked,
        recommended_ram: manifest.settings.recommended_ram,
    };

    let resolved = castpack::merge(base, overlay)?;

    if !resolved.blocked.is_empty() {
        reporter.set_blocked(resolved.blocked.clone());
    }

    Ok(CastPack {
        manifest,
        resolved,
        archive,
        base: base_pack,
    })
}

async fn read_manifest(url: &str, saved: &std::path::Path) -> CommandResult<Manifest> {
    match castpack::source::manifest(url).await {
        Ok(manifest) => Ok(manifest),
        Err(error) => match castpack::source::installed_manifest(saved).await {
            Some(manifest) => {
                eprintln!("Манифест сборки взят из сохранённого: {error}");
                Ok(manifest)
            }
            None => Err(error),
        },
    }
}

pub async fn sync(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    prepared: &CastPack,
) -> CommandResult<Instance> {
    let resolved = prepared.resolved();

    let loader = resolved.loader;
    let loader_version = resolved.loader_version.clone();
    let minecraft_version = resolved.minecraft_version.clone();
    let base = prepared.base.clone();
    let recommended_ram = resolved.recommended_ram;

    let loader_changed = instance.loader != loader;

    let updated = state
        .instances
        .update(paths, &instance.id, move |current| {
            current.loader = loader;
            current.minecraft_version = minecraft_version;
            current.loader_version = loader_version;
            current.pack = base;

            let Some(ram) = recommended_ram else { return };

            let fresh = current
                .castpack
                .as_ref()
                .is_some_and(|source| !source.ram_applied);

            if let Some(source) = current.castpack.as_mut() {
                source.ram_applied = true;
            }

            if fresh && !current.settings.override_memory && ram > 0 {
                current.settings.override_memory = true;
                current.settings.max_ram = ram;
            }
        })
        .await?;

    if loader_changed {
        cast_core::fs_util::remove_dir_if_exists(&paths.instance(&instance.id).natives()).await;
    }

    Ok(updated)
}

pub async fn apply(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    prepared: &CastPack,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    modpack::apply(
        state,
        paths,
        instance,
        modpack::Applied {
            resolved: prepared.resolved(),
            archive: prepared.archive.as_deref(),
            version_id: &prepared.manifest.version,
            phase: "castpack",
            label: "Файлы сборки",
        },
        reporter,
    )
    .await?;

    castpack::source::save_manifest(
        &paths.instance(&instance.id).castpack_manifest(),
        &prepared.manifest,
    )
    .await
}
