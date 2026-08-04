use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use tauri::AppHandle;

use cast_core::error::{CommandError, CommandResult};
use cast_core::instance::{Instance, LocalPackSource};
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
}

pub async fn inspect(path: &str) -> CommandResult<LocalPack> {
    let path = path.trim();

    if path.is_empty() {
        return Err(CommandError::fs("Не выбран файл модпака"));
    }

    local::inspect(Path::new(path)).await
}

pub async fn import(
    app: AppHandle,
    state: Arc<AppState>,
    request: FileImportRequest,
) -> CommandResult<Instance> {
    let pack = inspect(&request.path).await?;

    if let Some(reason) = &pack.blocked {
        return Err(CommandError::manifest(format!(
            "Модпак «{}» установить нельзя: {reason}",
            pack.name
        )));
    }

    let loader = pack
        .loader
        .ok_or_else(|| CommandError::manifest("В модпаке не указан загрузчик"))?;

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

async fn store_archive(paths: &LauncherPaths, instance_id: &str, source: &Path) -> CommandResult<()> {
    let target: PathBuf = paths.instance(instance_id).pack_archive();

    if let Some(parent) = target.parent() {
        cast_core::fs_util::ensure_dir(parent).await?;
    }

    tokio::fs::copy(source, &target)
        .await
        .map_err(|e| CommandError::io("Не удалось скопировать архив модпака", &target, e))?;

    Ok(())
}

fn text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
