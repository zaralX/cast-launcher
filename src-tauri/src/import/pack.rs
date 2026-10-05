use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use tauri::AppHandle;

use cast_core::castpack::file::{self as cast_file, ExistingPack};
use cast_core::castpack::{self, Target};
use cast_core::error::{CommandError, CommandResult};
use cast_core::icons;
use cast_core::instance::{CastPackSource, Instance, LocalPackKind, LocalPackSource};
use cast_core::packs::local::{self, LocalPack};
use cast_core::paths::LauncherPaths;

use crate::events::{EmitExt, LauncherEvent};
use crate::state::AppState;
use crate::telemetry::{self, Event};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileImportRequest {
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// A `.cast` replaces the pack of this instance instead of making a new one.
    #[serde(default)]
    pub update: Option<String>,
}

pub async fn inspect(state: &Arc<AppState>, path: &str) -> CommandResult<LocalPack> {
    let path = path.trim();

    if path.is_empty() {
        return Err(CommandError::invalid_input(
            "error.reason.modpack.file_required",
        ));
    }

    let mut pack = local::inspect(Path::new(path)).await?;

    if let Some(preview) = pack.cast.as_mut() {
        preview.existing = existing(state, &preview.id).await;
    }

    Ok(pack)
}

async fn existing(state: &Arc<AppState>, pack_id: &str) -> Option<ExistingPack> {
    state
        .instances
        .all()
        .await
        .into_iter()
        .find_map(|instance| {
            let source = instance.castpack.as_ref()?;

            (source.is_file() && source.catalog_id == pack_id).then(|| ExistingPack {
                instance_id: instance.id.clone(),
                name: instance.name.clone(),
                version: source.version.clone(),
                minecraft_version: instance.minecraft_version.clone(),
                loader: instance.loader,
            })
        })
}

pub async fn import(
    app: AppHandle,
    state: Arc<AppState>,
    request: FileImportRequest,
) -> CommandResult<Instance> {
    let pack = inspect(&state, &request.path).await?;

    if let Some(reason) = &pack.blocked {
        return Err(CommandError::unsupported("error.reason.modpack.blocked")
            .param("name", &pack.name)
            .param_text("reason", reason.clone()));
    }

    if pack.kind == LocalPackKind::Cast {
        return import_cast(app, state, request, pack).await;
    }

    let loader = pack
        .loader
        .ok_or_else(|| CommandError::invalid_input("error.reason.modpack.no_loader"))?;

    let instance = Instance {
        id: uuid::Uuid::new_v4().to_string(),
        name: text(request.name).unwrap_or_else(|| pack.name.clone()),
        description: text(request.description).unwrap_or_else(|| pack.description.clone()),
        minecraft_version: pack.minecraft_version.clone(),
        icon: String::new(),
        loader,
        installed: false,
        version: 1,
        loader_version: pack.loader_version.clone(),
        custom_id: None,
        pack: None,
        castpack: None,
        local_pack: Some(LocalPackSource {
            kind: pack.kind,
            name: pack.name.clone(),
            version: pack.version.clone(),
        }),
        cast_export: None,
        settings: pack.settings.clone(),
        playtime: Default::default(),
        dir: String::new(),
    };

    let paths = state.paths().await;
    let created = state.instances.create(&paths, instance).await?;

    if let Err(error) = store_archive(&paths, &created.id, Path::new(request.path.trim())).await {
        let _ = state.instances.remove(&paths, &created.id).await;
        return Err(error);
    }

    telemetry::track(&app, Event::new("instance_created").instance(&created));
    telemetry::track(
        &app,
        Event::new("pack_file_imported")
            .instance(&created)
            .text("kind", pack.kind.key())
            .num("files", pack.files as f64)
            .num("size_mb", telemetry::megabytes(pack.size)),
    );

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(created)
}

async fn import_cast(
    app: AppHandle,
    state: Arc<AppState>,
    request: FileImportRequest,
    pack: LocalPack,
) -> CommandResult<Instance> {
    let source = PathBuf::from(request.path.trim());
    let opened = cast_file::open(&source).await?;
    let manifest = &opened.file.manifest;

    // Resolving the base pack first refuses a file whose base cannot be installed.
    let target = castpack::target(manifest, castpack::base_pack(manifest).await?);
    let paths = state.paths().await;

    let update = text(request.update.clone());

    let instance = match update.as_deref() {
        Some(instance_id) => {
            update_from_file(&state, &paths, instance_id, &source, &opened).await?
        }
        None => create_from_file(&state, &paths, request, &source, &opened, target).await?,
    };

    let preview = pack
        .cast
        .unwrap_or_else(|| opened.file.preview(opened.icon.is_some()));

    log::info!(
        "Imported .cast '{}' {} into instance '{}' ({} catalog mods, {} embedded files, update: {})",
        manifest.name,
        manifest.version,
        instance.name,
        preview.modrinth_mods + preview.curseforge_mods,
        preview.embedded_files,
        update.is_some()
    );

    telemetry::track(
        &app,
        Event::new("cast_imported")
            .instance(&instance)
            .num(
                "mods",
                (preview.modrinth_mods + preview.curseforge_mods) as f64,
            )
            .num("embedded", preview.embedded_files as f64)
            .num("embedded_code", preview.embedded_code.len() as f64)
            .num("size_mb", telemetry::megabytes(pack.size))
            .flag("base", manifest.base.is_some())
            .flag("update", update.is_some()),
    );

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(instance)
}

async fn create_from_file(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    request: FileImportRequest,
    source: &Path,
    opened: &cast_file::Opened,
    target: Target,
) -> CommandResult<Instance> {
    let manifest = &opened.file.manifest;
    let icon = save_icon(paths, &manifest.name, opened).await;

    let instance = Instance {
        id: uuid::Uuid::new_v4().to_string(),
        name: text(request.name).unwrap_or_else(|| manifest.name.trim().to_string()),
        description: text(request.description)
            .unwrap_or_else(|| opened.file.description.trim().to_string()),
        minecraft_version: target.minecraft_version,
        icon: icon.unwrap_or_default(),
        loader: target.loader,
        installed: false,
        version: 1,
        loader_version: target.loader_version,
        custom_id: None,
        pack: target.pack,
        castpack: Some(CastPackSource::from_file(&manifest.id)),
        local_pack: None,
        cast_export: None,
        settings: Default::default(),
        playtime: Default::default(),
        dir: String::new(),
    };

    let created = state.instances.create(paths, instance).await?;

    if let Err(error) = store_archive(paths, &created.id, source).await {
        let _ = state.instances.remove(paths, &created.id).await;
        return Err(error);
    }

    Ok(created)
}

async fn update_from_file(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance_id: &str,
    source: &Path,
    opened: &cast_file::Opened,
) -> CommandResult<Instance> {
    let instance = state.instances.get(instance_id).await?;

    if !instance
        .castpack
        .as_ref()
        .is_some_and(|pack| pack.is_file())
    {
        return Err(
            CommandError::invalid_input("error.reason.cast.not_file_pack")
                .param("name", &instance.name),
        );
    }

    if state.processes.is_running(&instance.id).await {
        return Err(
            CommandError::conflict("error.reason.instance.already_running")
                .param("name", &instance.name),
        );
    }

    if state.installs.snapshot(&instance.id).await.is_some() {
        return Err(
            CommandError::conflict("error.reason.cast.update_while_installing")
                .param("name", &instance.name),
        );
    }

    store_archive(paths, &instance.id, source).await?;

    let icon = match instance.icon.trim().is_empty() {
        true => save_icon(paths, &opened.file.manifest.name, opened).await,
        false => None,
    };

    let pack_id = opened.file.manifest.id.clone();

    // The loader and the game version follow on install, the same way a catalog update does.
    state
        .instances
        .update(paths, &instance.id, move |current| {
            current.installed = false;

            if let Some(source) = current.castpack.as_mut() {
                source.catalog_id = pack_id;
            }

            if let Some(icon) = icon {
                current.icon = icon;
            }
        })
        .await
}

async fn save_icon(
    paths: &LauncherPaths,
    name: &str,
    opened: &cast_file::Opened,
) -> Option<String> {
    let icon = opened.icon.as_ref()?;
    let file_name = format!("{}.{}", name.trim(), icon.extension);

    match icons::save(&paths.icons(), &file_name, &icon.bytes).await {
        Ok(saved) => Some(saved.name),
        Err(error) => {
            log::warn!("Failed to save the icon of pack '{name}': {error}");
            None
        }
    }
}

/// Copies next to the target first: an update must not leave a half-copied pack behind.
async fn store_archive(
    paths: &LauncherPaths,
    instance_id: &str,
    source: &Path,
) -> CommandResult<()> {
    let target: PathBuf = paths.instance(instance_id).pack_archive();

    if let Some(parent) = target.parent() {
        cast_core::fs_util::ensure_dir(parent).await?;
    }

    let partial = target.with_extension(format!("{}.part", uuid::Uuid::new_v4().simple()));

    if let Err(error) = tokio::fs::copy(source, &partial).await {
        cast_core::fs_util::remove_file_if_exists(&partial).await;
        return Err(CommandError::io("error.reason.fs.copy", &target, error));
    }

    if let Err(error) = tokio::fs::rename(&partial, &target).await {
        cast_core::fs_util::remove_file_if_exists(&partial).await;
        return Err(CommandError::io("error.reason.fs.copy", &target, error));
    }

    Ok(())
}

fn text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
