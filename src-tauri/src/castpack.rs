use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;

use cast_core::castpack::{self, Catalog, CatalogPack, Manifest};
use cast_core::error::{CommandError, CommandResult};
use cast_core::icons;
use cast_core::instance::{CastPackSource, Instance, LoaderType, PackProvider, PackSource};
use cast_core::packs;

use crate::events::{EmitExt, LauncherEvent};
use crate::install;
use crate::state::AppState;
use crate::telemetry::{self, Event};

pub async fn catalog(app: &AppHandle, state: &Arc<AppState>) -> CommandResult<Catalog> {
    let config = state.config().await;
    let paths = state.paths().await;

    let catalog =
        castpack::source::catalog(config.launcher.catalog_url(), &castpack::source::catalog_cache(&paths))
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
                .is_some_and(|source| source.catalog_id == pack.id)
        }) else {
            continue;
        };

        if !instance.icon.trim().is_empty() {
            continue;
        }

        let Some(name) = save_icon(state, pack).await else { continue };

        let paths = state.paths().await;

        match state
            .instances
            .update(&paths, &instance.id, move |current| current.icon = name)
            .await
        {
            Ok(_) => healed = true,
            Err(error) => eprintln!("Иконка сборки «{}» не прописалась: {error}", pack.id),
        }
    }

    if healed {
        LauncherEvent::Instances {
            instances: state.instances.all().await,
        }
        .emit(app);
    }
}

pub async fn install_pack(
    app: AppHandle,
    state: Arc<AppState>,
    pack_id: &str,
) -> CommandResult<Instance> {
    let catalog = catalog(&app, &state).await?;

    let entry = catalog
        .find(pack_id)
        .ok_or_else(|| CommandError::manifest(format!("Сборки «{pack_id}» нет в каталоге")))?
        .clone();

    let manifest = castpack::source::manifest(&entry.manifest).await?;
    let base = base_pack(&manifest).await?;

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

    let (pack, loader, minecraft_version) = match base {
        Some((pack, loader, version)) => (Some(pack), loader, version),
        None => (
            None,
            manifest.loader().map(|(loader, _)| loader).unwrap_or(LoaderType::Vanilla),
            manifest.minecraft_version().unwrap_or_default().to_string(),
        ),
    };

    let (loader, loader_version) = match manifest.loader() {
        Some((loader, version)) => (loader, version),
        None => (loader, None),
    };

    let minecraft_version = manifest
        .minecraft_version()
        .map(str::to_string)
        .unwrap_or(minecraft_version);

    let icon = save_icon(state, entry).await;

    let existing = state.instances.get(&id).await.ok();

    let source = CastPackSource::new(&entry.id, &entry.manifest, entry.autoupdate);

    let instance = match existing {
        Some(_) => {
            let manifest_url = entry.manifest.clone();
            let name = entry.name.clone();
            let description = summary(entry);
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
                        description: summary(entry),
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

fn summary(entry: &CatalogPack) -> String {
    match entry.summary.trim().is_empty() {
        true => entry.description.trim().to_string(),
        false => entry.summary.trim().to_string(),
    }
}

pub async fn base_pack(
    manifest: &Manifest,
) -> CommandResult<Option<(PackSource, LoaderType, String)>> {
    let Some(spec) = &manifest.base else {
        return Ok(None);
    };

    let version = packs::version(spec.provider, &spec.project_id, &spec.version_id).await?;

    if let Some(reason) = version.unsupported_reason() {
        return Err(CommandError::manifest(format!(
            "Базовый модпак сборки установить нельзя: {reason}"
        )));
    }

    let (Some(loader), Some(minecraft_version), Some(file)) = (
        version.loader,
        version.minecraft_version.clone(),
        version.file.clone(),
    ) else {
        return Err(CommandError::manifest("У базового модпака сборки нет файла для скачивания"));
    };

    let pack = PackSource {
        provider: spec.provider,
        project_id: spec.project_id.clone(),
        version_id: version.id,
        version_number: version.version_number,
        file_url: file.url,
        file_name: file.filename,
        file_sha1: file.hashes.sha1,
        file_size: file.size,
    };

    Ok(Some((pack, loader, minecraft_version)))
}

async fn save_icon(state: &Arc<AppState>, entry: &CatalogPack) -> Option<String> {
    let url = entry.icon.as_deref()?;

    let bytes = match castpack::source::icon(url).await {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Иконка сборки «{}» не скачалась: {}", entry.id, error.message);
            return None;
        }
    };

    let paths = state.paths().await;
    let name = icon_name(&entry.id, url);

    match icons::save_once(&paths.icons(), &name, &bytes).await {
        Ok(icon) => Some(icon.name),
        Err(error) => {
            eprintln!("Иконка сборки «{}» не сохранилась: {}", entry.id, error.message);
            None
        }
    }
}

fn icon_name(pack_id: &str, url: &str) -> String {
    let extension = url
        .split('?')
        .next()
        .and_then(|path| path.rsplit('/').next())
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .filter(|extension| {
            (1..=5).contains(&extension.len()) && extension.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or_else(|| "png".to_string());

    format!("castpack-{pack_id}.{extension}")
}

pub async fn set_autoupdate(
    app: &AppHandle,
    state: &Arc<AppState>,
    instance_id: &str,
    enabled: bool,
) -> CommandResult<Instance> {
    let paths = state.paths().await;

    let updated = state
        .instances
        .update(&paths, instance_id, move |current| {
            if let Some(source) = current.castpack.as_mut() {
                source.autoupdate = enabled;
            }
        })
        .await?;

    if updated.castpack.is_none() {
        return Err(CommandError::manifest("Эта сборка не из каталога CastPack"));
    }

    telemetry::track(
        app,
        Event::new("castpack_autoupdate").instance(&updated).flag("enabled", enabled),
    );

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(app);

    Ok(updated)
}

pub async fn save_manifest_as(app: &AppHandle, json: &str) -> CommandResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;

    Manifest::parse(json.as_bytes())?;

    let (sender, receiver) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title("Сохранить манифест сборки")
        .set_file_name("manifest.json")
        .add_filter("JSON", &["json"])
        .save_file(move |picked| {
            let _ = sender.send(picked);
        });

    let Some(path) = receiver
        .await
        .ok()
        .flatten()
        .and_then(|picked| picked.into_path().ok())
    else {
        return Ok(None);
    };

    cast_core::fs_util::write_atomic(&path, json.as_bytes()).await?;

    Ok(Some(path.display().to_string()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbedMod {
    pub path: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    pub blocked: bool,
}

pub async fn probe_mod(
    provider: PackProvider,
    project_id: &str,
    version_id: &str,
) -> CommandResult<ProbedMod> {
    let entry = castpack::ModRef::Catalog {
        provider,
        project_id,
        version_id,
        optional: false,
    };

    let minecraft = std::path::Path::new("/");
    let resolved = castpack::mods::resolve(std::slice::from_ref(&entry), minecraft).await?;

    if let Some((path, task)) = resolved.files.into_iter().next() {
        return Ok(ProbedMod {
            path,
            url: task.url,
            sha1: task.sha1,
            size: task.size,
            blocked: false,
        });
    }

    let blocked = resolved
        .blocked
        .into_iter()
        .next()
        .ok_or_else(|| CommandError::manifest("Каталог не отдал такой файл"))?;

    Ok(ProbedMod {
        path: blocked.target_path,
        url: blocked.website_url,
        sha1: blocked.sha1,
        size: None,
        blocked: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_names_carry_the_pack_id_and_keep_the_extension() {
        assert_eq!(
            icon_name("zaralx-rpg", "https://cdn.zaralx.ru/icons/rpg.WEBP"),
            "castpack-zaralx-rpg.webp"
        );
        assert_eq!(
            icon_name("zaralx-rpg", "https://cdn.zaralx.ru/icons/rpg.png?v=2"),
            "castpack-zaralx-rpg.png"
        );
        assert_eq!(
            icon_name("zaralx-rpg", "https://cdn.zaralx.ru/icons/rpg"),
            "castpack-zaralx-rpg.png",
            "без расширения считаем png"
        );
    }

    #[test]
    fn the_card_description_falls_back_to_the_long_text() {
        let mut entry = CatalogPack {
            summary: "  ".into(),
            description: "  Длинное описание  ".into(),
            ..Default::default()
        };

        assert_eq!(summary(&entry), "Длинное описание");

        entry.summary = "Короткое".into();
        assert_eq!(summary(&entry), "Короткое");
    }
}
