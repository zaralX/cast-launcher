use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use cast_core::error::{CommandError, CommandResult};
use cast_core::icons;
use cast_core::import::copy::{CopyStats, Progress};
use cast_core::import::{
    prism, ImportOptions, ImportProgress, ImportReport, ImportStage, ImportedInstance, LauncherKind,
    SkippedInstance,
};
use cast_core::instance::{Instance, PackProvider, PackSource};
use cast_core::modrinth;
use cast_core::paths::LauncherPaths;

use crate::events::{EmitExt, LauncherEvent};
use crate::state::AppState;

const MODRINTH: &str = "modrinth";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLauncher {
    pub kind: LauncherKind,
    pub label: &'static str,
    pub path: String,
    pub instances: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub path: String,
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub options: ImportOptions,
}

pub fn detect() -> Vec<DetectedLauncher> {
    prism::detect()
        .map(|path| DetectedLauncher {
            kind: LauncherKind::Prism,
            label: LauncherKind::Prism.label(),
            instances: count_instances(&path),
            path: path.display().to_string(),
        })
        .into_iter()
        .collect()
}

fn count_instances(root: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(root.join(prism::INSTANCES)) else {
        return 0;
    };

    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(prism::CONFIG_FILE).is_file())
        .count()
}

pub async fn scan(path: &str) -> CommandResult<Vec<prism::ScannedInstance>> {
    prism::scan(&root_of(path)?).await
}

fn root_of(path: &str) -> CommandResult<PathBuf> {
    let path = path.trim();

    if path.is_empty() {
        return Err(CommandError::fs("Не указан каталог PrismLauncher"));
    }

    let root = prism::normalize(Path::new(path));

    if !prism::is_data_dir(&root) {
        return Err(CommandError::fs(format!(
            "Это не каталог данных PrismLauncher: внутри нет папки instances ({})",
            root.display()
        )));
    }

    Ok(root)
}

pub async fn run(
    app: AppHandle,
    state: Arc<AppState>,
    request: ImportRequest,
) -> CommandResult<ImportReport> {
    let _guard = state.imports.begin()?;

    let root = root_of(&request.path)?;
    let scanned = prism::scan(&root).await?;
    let paths = state.paths().await;

    let (selected, mut report) = cast_core::import::select(scanned, &request.folders);
    let total = selected.len() + usize::from(request.options.copies_shared());

    let publish = publisher(app.clone(), total);
    let cancelled = {
        let imports = Arc::clone(&state.imports);
        move || imports.is_cancelled()
    };

    let mut done = 0;

    if request.options.copies_shared() {
        match copy_shared(&root, &paths, &request.options, &publish, &cancelled).await {
            Ok(stats) => {
                report.stats = report.stats.plus(stats);
                done = 1;
            }
            Err(error) if error.is_aborted() => {
                report.cancelled = true;
                finish(&app, &state, report.stats, total).await;
                return Ok(report);
            }
            Err(error) => {
                finish(&app, &state, report.stats, total).await;
                return Err(error);
            }
        }
    }

    for instance in selected {
        if cancelled() {
            report.cancelled = true;
            break;
        }

        publish(ImportStage::Instances, &instance.name, done, report.stats);

        let step = Step {
            publish: &publish,
            name: &instance.name,
            done,
            base: report.stats,
        };

        match import_one(&state, &paths, &root, &instance, &request.options, &cancelled, &step).await {
            Ok((imported, stats)) => {
                report.stats = report.stats.plus(stats);
                report.imported.push(imported);
            }
            Err(error) if error.is_aborted() => {
                report.cancelled = true;
                break;
            }
            Err(error) => report.skipped.push(SkippedInstance {
                name: instance.name.clone(),
                reason: error.message,
            }),
        }

        done += 1;
    }

    finish(&app, &state, report.stats, total).await;

    Ok(report)
}

async fn copy_shared(
    root: &Path,
    paths: &LauncherPaths,
    options: &ImportOptions,
    publish: &(impl Fn(ImportStage, &str, usize, CopyStats) + Send + Sync),
    cancelled: &(impl Fn() -> bool + Send + Sync),
) -> CommandResult<CopyStats> {
    let step = std::sync::Mutex::new(String::from("Общие файлы"));

    let on_change = |stats: CopyStats| {
        let current = step.lock().map(|step| step.clone()).unwrap_or_default();
        publish(ImportStage::Shared, &current, 0, stats);
    };

    let progress = Progress::new(&on_change, cancelled);

    let targets = prism::SharedTargets {
        libraries: paths.libraries(),
        asset_indexes: paths.asset_indexes(),
        asset_objects: paths.asset_objects(),
        java_runtimes: paths.java_runtimes(),
    };

    prism::copy_shared(root, options, &targets, &progress, |name| {
        if let Ok(mut step) = step.lock() {
            *step = name.to_string();
        }

        publish(ImportStage::Shared, name, 0, progress.stats());
    })
    .await?;

    progress.flush();

    Ok(progress.stats())
}

struct Step<'a, P> {
    publish: &'a P,
    name: &'a str,
    done: usize,
    base: CopyStats,
}

async fn import_one<P>(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    root: &Path,
    scanned: &prism::ScannedInstance,
    options: &ImportOptions,
    cancelled: &(impl Fn() -> bool + Send + Sync),
    step: &Step<'_, P>,
) -> CommandResult<(ImportedInstance, CopyStats)>
where
    P: Fn(ImportStage, &str, usize, CopyStats) + Send + Sync,
{
    let on_change = |stats: CopyStats| {
        (step.publish)(
            ImportStage::Instances,
            step.name,
            step.done,
            step.base.plus(stats),
        );
    };

    let progress = Progress::new(&on_change, cancelled);

    let id = uuid::Uuid::new_v4().simple().to_string();
    let icon = match options.icons {
        true => copy_icon(root, paths, scanned, &progress).await,
        false => String::new(),
    };

    let mut instance = scanned.to_instance(id, icon)?;
    let linked = link_pack(&mut instance, scanned, options).await;

    let created = state.instances.create(paths, instance).await?;
    let instance_paths = paths.instance(&created.id);

    let targets = prism::InstanceTargets {
        minecraft: instance_paths.minecraft(),
        client_jar: instance_paths.client_jar(),
        loader_installer: prism::loader_installer_target(paths, &created),
    };

    if let Err(error) = prism::copy_instance(root, scanned, &targets, &progress).await {
        let _ = state.instances.remove(paths, &created.id).await;
        return Err(error);
    }

    Ok((
        ImportedInstance {
            id: created.id,
            name: created.name,
            linked,
        },
        progress.stats(),
    ))
}

async fn copy_icon(
    root: &Path,
    paths: &LauncherPaths,
    scanned: &prism::ScannedInstance,
    progress: &Progress<'_>,
) -> String {
    let Some(name) = &scanned.icon else { return String::new() };

    let source = root.join(prism::ICONS).join(name);
    let Ok(target) = icons::resolve(&paths.icons(), name) else { return String::new() };

    match cast_core::import::copy::copy_file(&source, &target, progress).await {
        Ok(_) if target.is_file() => name.clone(),
        _ => String::new(),
    }
}

async fn link_pack(
    instance: &mut Instance,
    scanned: &prism::ScannedInstance,
    options: &ImportOptions,
) -> bool {
    if !options.link_packs {
        return false;
    }

    let Some(pack) = &scanned.pack else { return false };

    if pack.provider != MODRINTH || pack.version_id.is_empty() {
        return false;
    }

    let Ok(version) = modrinth::version(&pack.version_id).await else { return false };
    let Some(file) = version.file else { return false };

    instance.pack = Some(PackSource {
        provider: PackProvider::Modrinth,
        project_id: pack.project_id.clone(),
        version_id: version.id,
        version_number: version.version_number,
        file_url: file.url,
        file_name: file.filename,
        file_sha1: file.hashes.sha1,
        file_size: file.size,
    });

    true
}

fn publisher(
    app: AppHandle,
    total: usize,
) -> impl Fn(ImportStage, &str, usize, CopyStats) + Send + Sync {
    move |stage, step, done, stats| {
        LauncherEvent::Import(ImportProgress {
            source: LauncherKind::Prism,
            stage,
            step: step.to_string(),
            done,
            total,
            stats,
        })
        .emit(&app);
    }
}

async fn finish(app: &AppHandle, state: &Arc<AppState>, stats: CopyStats, total: usize) {
    LauncherEvent::Import(ImportProgress {
        source: LauncherKind::Prism,
        stage: ImportStage::Done,
        step: String::new(),
        done: total,
        total,
        stats,
    })
    .emit(app);

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(app);
}
