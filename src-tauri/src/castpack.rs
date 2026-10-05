use std::sync::Arc;

use tauri::AppHandle;

use cast_core::castpack::{self, Catalog, CatalogPack, Manifest};
use cast_core::error::{CommandError, CommandResult};
use cast_core::icons;
use cast_core::instance::{CastPackSource, Instance, LoaderType, PackSource};

use crate::events::{EmitExt, LauncherEvent};
use crate::install;
use crate::state::AppState;
use crate::telemetry::{self, Event};

pub async fn catalog(app: &AppHandle, state: &Arc<AppState>) -> CommandResult<Catalog> {
    let config = state.config().await;
    let paths = state.paths().await;

    let catalog = castpack::source::catalog(
        config.launcher.catalog_url(),
        &castpack::source::catalog_cache(&paths),
    )
    .await
    .inspect_err(|error| {
        telemetry::track(
            app,
            Event::new("castpack_catalog_failed")
                .error(error)
                .text("host", telemetry::host_of(config.launcher.catalog_url())),
        )
    })?;

    heal_icons(app, state, &catalog).await;

    Ok(catalog)
}

async fn heal_icons(app: &AppHandle, state: &Arc<AppState>, catalog: &Catalog) {
    let installed = state.instances.all().await;
    let mut healed = false;

    for pack in &catalog.packs {
        if pack.icon.is_none() {
            continue;
        }

        let Some(instance) = installed.iter().find(|instance| {
            instance
                .castpack
                .as_ref()
                .is_some_and(|source| !source.is_file() && source.catalog_id == pack.id)
        }) else {
            continue;
        };

        if !instance.icon.trim().is_empty() {
            continue;
        }

        let Some(name) = save_icon(state, pack).await else {
            continue;
        };

        let paths = state.paths().await;

        match state
            .instances
            .update(&paths, &instance.id, move |current| current.icon = name)
            .await
        {
            Ok(_) => healed = true,
            Err(error) => log::warn!("Failed to set the icon of pack '{}': {error}", pack.id),
        }
    }

    if healed {
        LauncherEvent::Instances {
            instances: state.instances.all().await,
        }
        .emit(app);
    }
}

pub async fn manifest_url(
    state: &Arc<AppState>,
    instance_id: &str,
    source: &CastPackSource,
) -> String {
    let known = catalog_manifest_url(state, &source.catalog_id).await;
    let url = source.manifest_url_from(known.as_deref());

    if url == source.manifest_url {
        return url;
    }

    log::info!(
        "Pack '{}' moved: {} -> {url}",
        source.catalog_id,
        source.manifest_url
    );

    let paths = state.paths().await;
    let fresh = url.clone();

    if let Err(error) = state
        .instances
        .update(&paths, instance_id, move |current| {
            if let Some(source) = current.castpack.as_mut() {
                source.manifest_url = fresh;
            }
        })
        .await
    {
        log::warn!("Failed to store the new manifest URL in the instance: {error}");
    }

    url
}

async fn catalog_manifest_url(state: &Arc<AppState>, catalog_id: &str) -> Option<String> {
    if catalog_id.trim().is_empty() {
        return None;
    }

    let config = state.config().await;
    let paths = state.paths().await;

    let catalog = castpack::source::catalog(
        config.launcher.catalog_url(),
        &castpack::source::catalog_cache(&paths),
    )
    .await
    .inspect_err(|error| {
        log::warn!("CastPack catalog is unreadable, using the saved manifest URL: {error}")
    })
    .ok()?;

    Some(catalog.find(catalog_id)?.manifest.clone())
}

pub async fn install_pack(
    app: AppHandle,
    state: Arc<AppState>,
    pack_id: &str,
) -> CommandResult<Instance> {
    let catalog = catalog(&app, &state).await?;

    let entry = catalog
        .find(pack_id)
        .ok_or_else(|| {
            CommandError::not_found("error.reason.castpack.not_in_catalog").param("pack", pack_id)
        })?
        .clone();

    let manifest = castpack::source::manifest(&entry.manifest).await?;
    let base = castpack::base_pack(&manifest).await?;

    let instance = upsert(&app, &state, &entry, &manifest, base).await?;

    telemetry::track(
        &app,
        Event::new("castpack_install")
            .instance(&instance)
            .text("catalog_id", &entry.id)
            .text("version", &manifest.version)
            .flag("update", instance.installed),
    );

    install::start(app, state, instance.id.clone()).await?;

    Ok(instance)
}

async fn upsert(
    app: &AppHandle,
    state: &Arc<AppState>,
    entry: &CatalogPack,
    manifest: &Manifest,
    base: Option<(PackSource, LoaderType, String)>,
) -> CommandResult<Instance> {
    let paths = state.paths().await;
    let id = entry.instance_id();

    let castpack::Target {
        pack,
        loader,
        loader_version,
        minecraft_version,
    } = castpack::target(manifest, base);

    let icon = save_icon(state, entry).await;

    let existing = state.instances.get(&id).await.ok();

    let source = CastPackSource::new(&entry.id, &entry.manifest, entry.autoupdate);

    let instance = match existing {
        Some(_) => {
            let manifest_url = entry.manifest.clone();
            let name = entry.name.clone();
            let description = entry.card_description();
            let icon = icon.clone();
            let pack = pack.clone();

            state
                .instances
                .update(&paths, &id, move |current| {
                    current.name = name;
                    current.description = description;
                    current.loader = loader;
                    current.minecraft_version = minecraft_version;
                    current.loader_version = loader_version;
                    current.pack = pack;

                    if let Some(icon) = icon {
                        current.icon = icon;
                    }

                    match current.castpack.as_mut() {
                        Some(existing) => existing.manifest_url = manifest_url,
                        None => current.castpack = Some(source),
                    }
                })
                .await?
        }
        None => {
            state
                .instances
                .create(
                    &paths,
                    Instance {
                        id,
                        name: entry.name.clone(),
                        description: entry.card_description(),
                        minecraft_version,
                        icon: icon.unwrap_or_default(),
                        loader,
                        installed: false,
                        version: 1,
                        loader_version,
                        custom_id: None,
                        pack,
                        castpack: Some(source),
                        local_pack: None,
                        cast_export: None,
                        settings: Default::default(),
                        playtime: Default::default(),
                        dir: String::new(),
                    },
                )
                .await?
        }
    };

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(app);

    Ok(instance)
}

async fn save_icon(state: &Arc<AppState>, entry: &CatalogPack) -> Option<String> {
    let url = entry.icon.as_deref()?;

    let bytes = match castpack::source::icon(url).await {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!(
                "Failed to download the icon of pack '{}': {}",
                entry.id,
                error
            );
            return None;
        }
    };

    let paths = state.paths().await;
    let name = entry.icon_file_name(url);

    match icons::save_once(&paths.icons(), &name, &bytes).await {
        Ok(icon) => Some(icon.name),
        Err(error) => {
            log::warn!("Failed to save the icon of pack '{}': {}", entry.id, error);
            None
        }
    }
}

pub async fn set_autoupdate(
    app: &AppHandle,
    state: &Arc<AppState>,
    instance_id: &str,
    enabled: bool,
) -> CommandResult<Instance> {
    let paths = state.paths().await;

    let instance = state.instances.get(instance_id).await?;

    match &instance.castpack {
        None => {
            return Err(CommandError::invalid_input(
                "error.reason.castpack.not_castpack",
            ))
        }
        Some(source) if source.is_file() => {
            return Err(CommandError::invalid_input(
                "error.reason.cast.no_autoupdate",
            ))
        }
        Some(_) => {}
    }

    let updated = state
        .instances
        .update(&paths, instance_id, move |current| {
            if let Some(source) = current.castpack.as_mut() {
                source.autoupdate = enabled;
            }
        })
        .await?;

    telemetry::track(
        app,
        Event::new("castpack_autoupdate")
            .instance(&updated)
            .flag("enabled", enabled),
    );

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(app);

    Ok(updated)
}
