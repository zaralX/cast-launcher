use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;

use cast_core::castpack::{self, Manifest};
use cast_core::error::{CommandError, CommandResult};
use cast_core::install::pack_files::PackFiles;
use cast_core::instance::Instance;
use cast_core::launch::game::RunningGame;
use cast_core::paths::LauncherPaths;

use crate::install::{self, InstallSnapshot};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PlayOutcome {
    Launched { game: RunningGame },
    Installing { install: InstallSnapshot },
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CastPackUpdate {
    pub available: bool,
    pub version: String,
    pub changelog: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn play(
    app: AppHandle,
    state: Arc<AppState>,
    instance_id: &str,
) -> CommandResult<PlayOutcome> {
    let instance = state.instances.get(instance_id).await?;

    if state.processes.is_running(&instance.id).await {
        return Err(CommandError::launch(format!(
            "Сборка «{}» уже запущена",
            instance.name
        )));
    }

    if let Some(install) = state.installs.snapshot(&instance.id).await {
        return Ok(PlayOutcome::Installing { install });
    }

    let paths = state.paths().await;

    if needs_install(&paths, &instance).await {
        let install = install::start_with(app, state, instance.id.clone(), true).await?;

        return Ok(PlayOutcome::Installing { install });
    }

    let game = crate::launch::launch(app, state, &instance.id).await?;

    Ok(PlayOutcome::Launched { game })
}

async fn needs_install(paths: &LauncherPaths, instance: &Instance) -> bool {
    if !instance.installed {
        return true;
    }

    if let Some(source) = &instance.castpack {
        if source.autoupdate {
            match castpack::source::manifest(&source.manifest_url).await {
                Ok(manifest) if source.is_outdated(&manifest.version) => return true,
                Ok(_) => {}
                Err(error) => {
                    eprintln!("Обновление сборки «{}» не проверено: {error}", instance.name);
                }
            }
        }
    }

    !missing_files(paths, instance).await.is_empty()
}

async fn missing_files(paths: &LauncherPaths, instance: &Instance) -> Vec<String> {
    if instance.pack.is_none() && instance.castpack.is_none() {
        return Vec::new();
    }

    let instance_paths = paths.instance(&instance.id);
    let record = PackFiles::load(&instance_paths.pack_files()).await;

    let missing = record.missing(&instance_paths.minecraft()).await;

    if !missing.is_empty() {
        eprintln!(
            "У сборки «{}» не хватает файлов ({}), переустанавливаю: {}",
            instance.name,
            missing.len(),
            missing.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
        );
    }

    missing
}

pub async fn check_update(state: &Arc<AppState>, instance_id: &str) -> CommandResult<CastPackUpdate> {
    let instance = state.instances.get(instance_id).await?;

    let source = instance
        .castpack
        .as_ref()
        .ok_or_else(|| CommandError::manifest("Эта сборка не из каталога CastPack"))?;

    match castpack::source::manifest(&source.manifest_url).await {
        Ok(manifest) => Ok(CastPackUpdate {
            available: source.is_outdated(&manifest.version),
            version: manifest.version,
            changelog: manifest.changelog,
            error: None,
        }),
        Err(error) => Ok(CastPackUpdate {
            available: false,
            version: source.version.clone(),
            changelog: source.changelog.clone(),
            error: Some(error.message),
        }),
    }
}

pub fn parse_manifest(json: &str) -> CommandResult<Manifest> {
    Manifest::parse(json.as_bytes())
}
