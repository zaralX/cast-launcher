use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use cast_core::error::CommandResult;
use cast_core::icons;
use cast_core::import::copy::{CopyStats, Progress};
use cast_core::import::{
    modrinth, prism, ImportOptions, ImportProgress, ImportReport, ImportStage, ImportedInstance,
    InstanceTargets, LauncherKind, ScannedInstance, SharedTargets, SkippedInstance, Source,
};
use cast_core::instance::{Instance, PackProvider, PackSource};
use cast_core::packs;
use cast_core::paths::LauncherPaths;

use crate::events::{EmitExt, LauncherEvent};
use crate::state::AppState;

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
    pub kind: LauncherKind,
    pub path: String,
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub options: ImportOptions,
}

pub async fn detect() -> Vec<DetectedLauncher> {
    let mut found = Vec::new();

    for kind in LauncherKind::ALL {
        let Some(path) = cast_core::import::detect(kind) else {
            continue;
        };

        found.push(DetectedLauncher {
            kind,
            label: kind.label(),
            instances: count_instances(kind, &path).await,
            path: path.display().to_string(),
        });
    }

    found
}

async fn count_instances(kind: LauncherKind, path: &Path) -> usize {
    match kind {
        LauncherKind::Prism => {
            let Ok(entries) = std::fs::read_dir(path.join(prism::INSTANCES)) else {
                return 0;
            };

            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().join(prism::CONFIG_FILE).is_file())
                .count()
        }
        LauncherKind::Modrinth => modrinth::open(path).await.map(|root| root.instances()).unwrap_or(0),
    }
}

pub async fn scan(kind: LauncherKind, path: &str) -> CommandResult<Vec<ScannedInstance>> {
    Source::open(kind, path).await?.scan().await
}

pub async fn run(
    app: AppHandle,
    state: Arc<AppState>,
    request: ImportRequest,
) -> CommandResult<ImportReport> {
    let _guard = state.imports.begin()?;

    let source = Source::open(request.kind, &request.path).await?;
    let scanned = source.scan().await?;
    let paths = state.paths().await;

    let (selected, mut report) = cast_core::import::select(scanned, &request.folders);
    let total = selected.len() + usize::from(request.options.copies_shared());

    let publish = publisher(app.clone(), request.kind, total);
    let cancelled = {
        let imports = Arc::clone(&state.imports);
        move || imports.is_cancelled()
    };

    let mut done = 0;

    if request.options.copies_shared() {
        match copy_shared(&source, &paths, &request.options, &publish, &cancelled).await {
            Ok(stats) => {
                report.stats = report.stats.plus(stats);
                done = 1;
            }
            Err(error) if error.is_aborted() => {
                report.cancelled = true;
                finish(&app, &state, request.kind, report.stats, total).await;
                return Ok(report);
            }
            Err(error) => {
                finish(&app, &state, request.kind, report.stats, total).await;
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

        match import_one(&state, &paths, &source, &instance, &request.options, &cancelled, &step).await {
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

    finish(&app, &state, request.kind, report.stats, total).await;

    Ok(report)
}

async fn copy_shared(
    source: &Source,
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
    let targets = SharedTargets::of(paths);

    source
        .copy_shared(options, &targets, &progress, |name| {
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
    source: &Source,
    scanned: &ScannedInstance,
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
        true => copy_icon(paths, scanned, &progress).await,
        false => String::new(),
    };

    let mut instance = scanned.to_instance(id, icon)?;
    let linked = link_pack(&mut instance, scanned, options).await;

    let created = state.instances.create(paths, instance).await?;
    let instance_paths = paths.instance(&created.id);

    let targets = InstanceTargets {
        minecraft: instance_paths.minecraft(),
        client_jar: instance_paths.client_jar(),
        loader_installer: source.loader_installer_target(paths, &created),
    };

    if let Err(error) = source.copy_instance(scanned, &targets, &progress).await {
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
    paths: &LauncherPaths,
    scanned: &ScannedInstance,
    progress: &Progress<'_>,
) -> String {
    let (Some(name), Some(source)) = (&scanned.icon, &scanned.icon_source) else {
        return String::new();
    };

    let Ok(target) = icons::resolve(&paths.icons(), name) else {
        return String::new();
    };

    match cast_core::import::copy::copy_file(source, &target, progress).await {
        Ok(_) if target.is_file() => name.clone(),
        _ => String::new(),
    }
}

async fn link_pack(
    instance: &mut Instance,
    scanned: &ScannedInstance,
    options: &ImportOptions,
) -> bool {
    if !options.link_packs {
        return false;
    }

    let Some(pack) = &scanned.pack else { return false };

    let Some(provider) = PackProvider::from_key(&pack.provider) else { return false };

    if pack.version_id.is_empty() {
        return false;
    }

    let Ok(version) = packs::version(provider, &pack.project_id, &pack.version_id).await else {
        return false;
    };

    let Some(file) = version.file else { return false };

    instance.pack = Some(PackSource {
        provider,
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
    source: LauncherKind,
    total: usize,
) -> impl Fn(ImportStage, &str, usize, CopyStats) + Send + Sync {
    move |stage, step, done, stats| {
        LauncherEvent::Import(ImportProgress {
            source,
            stage,
            step: step.to_string(),
            done,
            total,
            stats,
        })
        .emit(&app);
    }
}

async fn finish(
    app: &AppHandle,
    state: &Arc<AppState>,
    source: LauncherKind,
    stats: CopyStats,
    total: usize,
) {
    LauncherEvent::Import(ImportProgress {
        source,
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
