use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use cast_core::castpack::export::{
    self, BaseFiles, Build, Collected, ExportBase, ExportFile, ExportProgress, ExportRequest,
    ExportResult, ExportScan, ExportStage, Lookup,
};
use cast_core::castpack::file::{self as cast_file, BaseInfo, Embed, Icon, WriteOptions};
use cast_core::castpack::manifest::BaseSpec;
use cast_core::error::{CommandError, CommandResult};
use cast_core::icons;
use cast_core::instance::{CastExport, Instance};
use cast_core::mods::hash::FileHashes;
use cast_core::mods::{self, ModsScan};
use cast_core::net::download::{DownloadOptions, DownloadTask, JobSnapshot};
use cast_core::packs;
use cast_core::packs::export::{self as modpack, ExportFormat};
use cast_core::paths::LauncherPaths;

use crate::commands::DialogText;
use crate::events::{EmitExt, LauncherEvent};
use crate::install;
use crate::state::AppState;
use crate::telemetry::{self, Event};

const PROGRESS_EVERY: Duration = Duration::from_millis(150);

#[derive(Default)]
pub struct ExportRegistry {
    running: AtomicBool,
    cancel: AtomicBool,
}

impl ExportRegistry {
    fn claim(&self) -> Option<ExportGuard<'_>> {
        self.running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()?;

        self.cancel.store(false, Ordering::SeqCst);

        Some(ExportGuard(self))
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

struct ExportGuard<'a>(&'a ExportRegistry);

impl Drop for ExportGuard<'_> {
    fn drop(&mut self) {
        self.0.cancel.store(false, Ordering::SeqCst);
        self.0.running.store(false, Ordering::SeqCst);
    }
}

pub async fn scan(
    app: &AppHandle,
    state: &Arc<AppState>,
    instance_id: &str,
) -> CommandResult<ExportScan> {
    let instance = state.instances.get(instance_id).await?;
    let paths = state.paths().await;
    let minecraft = paths.instance(&instance.id).minecraft();
    let progress = Progress::new(app, &instance.id);
    let started = Instant::now();

    let mut tree = {
        let minecraft = minecraft.clone();

        tokio::task::spawn_blocking(move || export::scan_tree(&minecraft))
            .await
            .map_err(|e| CommandError::task_panicked("scan_export_tree", e))??
    };

    let tree_time = started.elapsed();
    let candidates = export::candidates(&tree, &minecraft);
    let lookup = identify(state, &paths, &instance, &candidates, &progress).await?;

    export::mark_sources(&mut tree, &lookup);

    log::info!(
        "Export dialog for '{}' read in {:.1}s: tree {:.1}s, catalogs {:.1}s",
        instance.name,
        started.elapsed().as_secs_f64(),
        tree_time.as_secs_f64(),
        (started.elapsed() - tree_time).as_secs_f64()
    );

    let base = match &instance.pack {
        Some(pack) => Some(ExportBase {
            provider: pack.provider,
            name: match packs::cached_archive(&paths, pack) {
                Ok(archive) if archive.is_file() => base_name(&archive).await,
                _ => String::new(),
            },
            version: pack.version_number.clone(),
        }),
        None => None,
    };

    Ok(ExportScan {
        tree,
        defaults: export::defaults(&instance),
        unchecked: lookup.unchecked.len(),
        base,
    })
}

async fn base_name(archive: &Path) -> String {
    packs::local::inspect(archive)
        .await
        .map(|found| found.name)
        .unwrap_or_default()
}

fn mods_scan(paths: &LauncherPaths, instance: &Instance) -> ModsScan {
    let dirs = paths.instance(&instance.id);

    ModsScan {
        dir: dirs.mods(),
        index_file: dirs.mods_index(),
        icons: paths.mod_icons(),
        loader: mods::ModLoader::of(instance.loader),
        managed: Default::default(),
    }
}

async fn lookup(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    files: &[(String, PathBuf)],
    progress: &Progress,
) -> CommandResult<Lookup> {
    let lock = state.mods.of(&instance.id).await;
    let _guard = lock.lock().await;

    let minecraft = paths.instance(&instance.id).minecraft();
    let scan = mods_scan(paths, instance);

    export::lookup(
        files,
        &scan,
        &minecraft,
        &paths.mod_catalog(),
        &|done, total| progress.hashed(done, total),
    )
    .await
}

async fn identify(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    files: &[(String, PathBuf)],
    progress: &Progress,
) -> CommandResult<Lookup> {
    let lock = state.mods.of(&instance.id).await;
    let _guard = lock.lock().await;

    export::identify(
        files,
        &mods_scan(paths, instance),
        &paths.mod_catalog(),
        &|done, total| progress.hashed(done, total),
    )
    .await
}

async fn file_hashes(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    files: &[(String, PathBuf)],
    progress: &Progress,
) -> CommandResult<BTreeMap<String, FileHashes>> {
    let lock = state.mods.of(&instance.id).await;
    let _guard = lock.lock().await;

    export::hash_files(files, &mods_scan(paths, instance), &|done, total| {
        progress.hashed(done, total)
    })
    .await
}

/// Sha1 of each file, empty when it could not be read: such a file never counts as unchanged.
async fn local_hashes(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    files: &[(String, PathBuf)],
    progress: &Progress,
) -> CommandResult<BTreeMap<String, String>> {
    let hashes = file_hashes(state, paths, instance, files, progress).await?;

    Ok(files
        .iter()
        .map(|(key, _)| {
            let sha1 = hashes
                .get(key)
                .map(|found| found.sha1.clone())
                .unwrap_or_default();

            (key.clone(), sha1)
        })
        .collect())
}

struct Base {
    spec: BaseSpec,
    info: BaseInfo,
    files: BaseFiles,
}

/// What the modpack the instance is built on brings: its downloads, manual downloads and
/// overrides, read from its archive, which is downloaded first when the cache lost it.
async fn base(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    progress: &Arc<Progress>,
) -> CommandResult<Option<Base>> {
    let Some(pack) = &instance.pack else {
        return Ok(None);
    };

    let archive = packs::cached_archive(paths, pack)?;

    if !archive.is_file() {
        if pack.file_url.trim().is_empty() {
            return Err(CommandError::unsupported("error.reason.cast.base_manual")
                .param("version", &pack.version_number));
        }

        progress.send(ExportStage::Downloading, 0, pack.file_size.unwrap_or(0));

        let sink = {
            let progress = Arc::clone(progress);

            Box::new(move |snapshot: &JobSnapshot| {
                progress.throttled(
                    ExportStage::Downloading,
                    snapshot.downloaded_bytes,
                    snapshot.total_bytes,
                )
            })
        };

        state
            .downloads
            .run(
                install::job_id(&instance.id, "cast-export-base"),
                vec![DownloadTask::verified(
                    pack.file_url.clone(),
                    archive.clone(),
                    pack.file_size,
                    pack.file_sha1.clone(),
                )],
                DownloadOptions::default(),
                Some(sink),
            )
            .await?;
    }

    progress.send(ExportStage::Modpack, 0, 0);

    let started = Instant::now();
    let minecraft = paths.instance(&instance.id).minecraft();
    let resolved = packs::local::resolve(&archive, &minecraft).await?;
    let resolved_in = started.elapsed();

    let overrides = {
        let progress = Arc::clone(progress);

        cast_core::archive::entry_hashes(
            archive.clone(),
            resolved.overrides.clone(),
            move |done, total| progress.throttled(ExportStage::Modpack, done, total),
        )
        .await?
    };

    let mut files = BaseFiles::new();

    for (key, task) in resolved.paths.iter().zip(&resolved.tasks) {
        files.insert(key.clone(), task.sha1.clone());
    }

    for file in &resolved.blocked {
        files.insert(file.target_path.clone(), file.sha1.clone());
    }

    // Overrides are unpacked after the downloads and win on the same path.
    files.extend(overrides.into_iter().map(|(key, sha1)| (key, Some(sha1))));

    log::info!(
        "Read the base modpack of '{}': {} files, resolved in {:.1}s, overrides hashed in {:.1}s",
        instance.name,
        files.len(),
        resolved_in.as_secs_f64(),
        (started.elapsed() - resolved_in).as_secs_f64()
    );

    Ok(Some(Base {
        spec: BaseSpec {
            provider: pack.provider,
            project_id: pack.project_id.clone(),
            version_id: pack.version_id.clone(),
        },
        info: BaseInfo {
            name: base_name(&archive).await,
            version: pack.version_number.clone(),
        },
        files,
    }))
}

struct Job<'a> {
    state: &'a Arc<AppState>,
    paths: &'a LauncherPaths,
    instance: &'a Instance,
    request: &'a ExportRequest,
    partial: &'a Path,
    progress: Arc<Progress>,
}

struct Packed {
    links: usize,
    embedded: usize,
    embedded_code: usize,
    removed: usize,
    unchecked: usize,
    id: Option<String>,
    with_base: bool,
}

pub async fn export(
    app: &AppHandle,
    state: &Arc<AppState>,
    instance_id: &str,
    request: ExportRequest,
    dialog: DialogText,
) -> CommandResult<Option<ExportResult>> {
    let request = request.normalized()?;
    let instance = state.instances.get(instance_id).await?;

    if state.installs.snapshot(&instance.id).await.is_some() {
        return Err(
            CommandError::conflict("error.reason.cast.export_while_installing")
                .param("name", &instance.name),
        );
    }

    let Some(_guard) = state.exports.claim() else {
        return Err(CommandError::conflict("error.reason.cast.export_running"));
    };

    let Some(target) = pick_target(app, dialog, &request).await else {
        return Ok(None);
    };

    let started = Instant::now();
    let paths = state.paths().await;
    let minecraft = paths.instance(&instance.id).minecraft();

    let progress = Progress::new(app, &instance.id);

    progress.send(ExportStage::Collecting, 0, 0);

    let Collected { files, skipped } = {
        let include = request.include.clone();
        let progress = Arc::clone(&progress);

        tokio::task::spawn_blocking(move || {
            export::collect(&minecraft, &include, &|found| {
                progress.throttled(ExportStage::Collecting, found, 0)
            })
        })
        .await
        .map_err(|e| CommandError::task_panicked("collect_export_files", e))??
    };

    cancelled(state)?;

    let partial = partial_path(&target);
    let job = Job {
        state,
        paths: &paths,
        instance: &instance,
        request: &request,
        partial: &partial,
        progress: Arc::clone(&progress),
    };

    let packed = match request.format {
        ExportFormat::Cast => pack_cast(&job, files).await,
        ExportFormat::Mrpack => pack_mrpack(&job, files).await,
        ExportFormat::CurseForge => pack_curseforge(&job, files).await,
    };

    let packed = match packed {
        Ok(packed) => packed,
        Err(error) => {
            cast_core::fs_util::remove_file_if_exists(&partial).await;
            return Err(error);
        }
    };

    if let Err(error) = tokio::fs::rename(&partial, &target).await {
        cast_core::fs_util::remove_file_if_exists(&partial).await;
        return Err(CommandError::io(
            "error.reason.fs.write_file",
            &target,
            error,
        ));
    }

    let size = tokio::fs::metadata(&target)
        .await
        .map(|meta| meta.len())
        .unwrap_or(0);

    let result = ExportResult {
        path: target.display().to_string(),
        size,
        mods: packed.links,
        embedded: packed.embedded,
        embedded_code: packed.embedded_code,
        removed: packed.removed,
        skipped,
    };

    if let Some(id) = packed.id {
        remember(state, &paths, &instance, id, &request).await;
    }

    log::info!(
        "Exported instance '{}' as {} {} ({}): {} catalog files, {} embedded ({} mods), {} removed from the base, {} bytes",
        instance.name,
        request.name,
        request.version,
        request.format.name(),
        result.mods,
        result.embedded,
        result.embedded_code,
        result.removed,
        result.size
    );
    log::info!("Export stages: {}", progress.timings());

    telemetry::track(
        app,
        Event::new("cast_exported")
            .instance(&instance)
            .text("format", request.format.name())
            .num("mods", result.mods as f64)
            .num("embedded", result.embedded as f64)
            .num("embedded_code", result.embedded_code as f64)
            .num("unchecked", packed.unchecked as f64)
            .num("size_mb", telemetry::megabytes(result.size))
            .num("removed", result.removed as f64)
            .num("duration_s", started.elapsed().as_secs_f64())
            .flag("base", packed.with_base)
            .flag("known_pack", export::pack_id(&instance).is_some()),
    );

    Ok(Some(result))
}

async fn pack_cast(job: &Job<'_>, files: Vec<ExportFile>) -> CommandResult<Packed> {
    let Job {
        state,
        paths,
        instance,
        request,
        ..
    } = *job;

    let base = match request.use_base {
        true => base(state, paths, instance, &job.progress).await?,
        false => None,
    };

    cancelled(state)?;

    let (files, diff) = match &base {
        Some(base) => {
            let relevant: Vec<(String, PathBuf)> = files
                .iter()
                .filter(|file| base.files.contains_key(&file.key))
                .map(|file| (file.key.clone(), file.path.clone()))
                .collect();

            let local = local_hashes(state, paths, instance, &relevant, &job.progress).await?;
            let diff = export::diff(&base.files, &local, export::covered_by(&request.include));

            let files = files
                .into_iter()
                .filter(|file| !diff.skip.contains(&file.key))
                .collect();

            (files, Some(diff))
        }
        None => (files, None),
    };

    let candidates: Vec<(String, PathBuf)> = files
        .iter()
        .map(|file| (file.key.clone(), file.path.clone()))
        .collect();

    let lookup = lookup(state, paths, instance, &candidates, &job.progress).await?;

    cancelled(state)?;

    let delete: Vec<String> = match &diff {
        Some(diff) => diff
            .delete
            .iter()
            .cloned()
            .chain(export::stale_replacements(diff, &lookup))
            .collect(),
        None => Vec::new(),
    };

    let split = export::split(files, &lookup)?;
    let id = export::pack_id(instance).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let links = split.mods.len();
    let removed = delete.len();
    let with_base = base.is_some();

    let file = export::build(Build {
        instance,
        request,
        id: id.clone(),
        mods: split.mods,
        exported_by: env!("CARGO_PKG_VERSION"),
        base: base.map(|base| (base.spec, base.info)),
        delete,
    });

    let embeds: Vec<Embed> = split
        .embedded
        .iter()
        .map(|file| Embed {
            key: file.key.clone(),
            source: file.path.clone(),
            mode: file.mode,
        })
        .collect();

    let total: u64 = split.embedded.iter().map(|file| file.size).sum();
    let icon = read_icon(paths, instance).await;

    job.progress.send(ExportStage::Packing, 0, total);

    let written = {
        let state = Arc::clone(state);
        let progress = Arc::clone(&job.progress);
        let partial = job.partial.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let report = |done: u64| progress.throttled(ExportStage::Packing, done, total);
            let cancelled = || state.exports.is_cancelled();

            cast_file::write(
                &partial,
                file,
                embeds,
                WriteOptions {
                    icon: icon.as_ref(),
                    progress: &report,
                    cancelled: &cancelled,
                },
            )
        })
        .await
        .map_err(|e| CommandError::task_panicked("write_cast_file", e))??
    };

    let preview = written.preview(false);

    Ok(Packed {
        links,
        embedded: preview.embedded_files,
        embedded_code: preview.embedded_code.len(),
        removed,
        unchecked: lookup.unchecked.len(),
        id: Some(id),
        with_base,
    })
}

async fn pack_mrpack(job: &Job<'_>, files: Vec<ExportFile>) -> CommandResult<Packed> {
    let hashes = catalog_hashes(job, &files).await?;
    let links = modpack::modrinth_links(&hashes).await?;

    write_modpack(
        job,
        modpack::mrpack(job.instance, job.request, files, &links)?,
    )
    .await
}

async fn pack_curseforge(job: &Job<'_>, files: Vec<ExportFile>) -> CommandResult<Packed> {
    let hashes = catalog_hashes(job, &files).await?;
    let minecraft = job.paths.instance(&job.instance.id).minecraft();
    let links = modpack::curseforge_links(&hashes, &minecraft).await?;

    write_modpack(
        job,
        modpack::curseforge(job.instance, job.request, files, &links)?,
    )
    .await
}

async fn catalog_hashes(
    job: &Job<'_>,
    files: &[ExportFile],
) -> CommandResult<BTreeMap<String, FileHashes>> {
    let candidates: Vec<(String, PathBuf)> = files
        .iter()
        .filter(|file| export::is_catalog_candidate(&file.key))
        .map(|file| (file.key.clone(), file.path.clone()))
        .collect();

    job.progress
        .send(ExportStage::Hashing, 0, candidates.len() as u64);

    let hashes = file_hashes(
        job.state,
        job.paths,
        job.instance,
        &candidates,
        &job.progress,
    )
    .await?;

    cancelled(job.state)?;

    Ok(hashes)
}

async fn write_modpack(job: &Job<'_>, plan: modpack::Plan) -> CommandResult<Packed> {
    let total = plan.size();
    let packed = Packed {
        links: plan.links,
        embedded: plan.overrides.len(),
        embedded_code: plan.embedded_code(),
        removed: 0,
        unchecked: 0,
        id: None,
        with_base: false,
    };

    job.progress.send(ExportStage::Packing, 0, total);

    let state = Arc::clone(job.state);
    let progress = Arc::clone(&job.progress);
    let partial = job.partial.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let report = |done: u64| progress.throttled(ExportStage::Packing, done, total);
        let cancelled = || state.exports.is_cancelled();

        modpack::write(&partial, &plan, &report, &cancelled)
    })
    .await
    .map_err(|e| CommandError::task_panicked("write_modpack", e))??;

    Ok(packed)
}

fn cancelled(state: &Arc<AppState>) -> CommandResult<()> {
    match state.exports.is_cancelled() {
        true => Err(CommandError::aborted("error.reason.cast.export_cancelled")),
        false => Ok(()),
    }
}

/// The next export of this instance starts from what this one used.
async fn remember(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    id: String,
    request: &ExportRequest,
) {
    let export = CastExport {
        id,
        version: request.version.clone(),
        author: request.author.clone(),
    };

    if let Err(error) = state
        .instances
        .update(paths, &instance.id, move |current| {
            current.cast_export = Some(export)
        })
        .await
    {
        log::warn!("The pack is exported, but its id was not saved in the instance: {error}");
    }
}

async fn pick_target(
    app: &AppHandle,
    dialog: DialogText,
    request: &ExportRequest,
) -> Option<PathBuf> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let extension = request.format.extension();

    app.dialog()
        .file()
        .set_title(dialog.title)
        .add_filter(dialog.filter, &[extension])
        .set_file_name(cast_file::file_name(
            &request.name,
            &request.version,
            extension,
        ))
        .save_file(move |picked| {
            let _ = sender.send(picked);
        });

    let picked = receiver.await.ok().flatten()?.into_path().ok()?;

    let has_extension = picked
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension));

    Some(match has_extension {
        true => picked,
        false => {
            let mut name = picked.into_os_string();
            name.push(format!(".{extension}"));
            PathBuf::from(name)
        }
    })
}

fn partial_path(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
}

async fn read_icon(paths: &LauncherPaths, instance: &Instance) -> Option<Icon> {
    let name = instance.icon.trim();

    if name.is_empty() {
        return None;
    }

    let path = icons::resolve(&paths.icons(), name).ok()?;
    let extension = path.extension()?.to_string_lossy().to_lowercase();

    if tokio::fs::metadata(&path).await.ok()?.len() > icons::MAX_SIZE {
        return None;
    }

    let bytes = tokio::fs::read(&path).await.ok()?;

    Some(Icon { extension, bytes })
}

struct Progress {
    app: AppHandle,
    instance_id: String,
    last: Mutex<Option<Instant>>,
    stages: Mutex<Vec<(ExportStage, Instant)>>,
}

impl Progress {
    fn new(app: &AppHandle, instance_id: &str) -> Arc<Self> {
        Arc::new(Self {
            app: app.clone(),
            instance_id: instance_id.to_string(),
            last: Mutex::new(None),
            stages: Mutex::new(Vec::new()),
        })
    }

    fn send(&self, stage: ExportStage, done: u64, total: u64) {
        let now = Instant::now();

        *lock(&self.last) = Some(now);

        {
            let mut stages = lock(&self.stages);

            if stages.last().is_none_or(|(last, _)| *last != stage) {
                stages.push((stage, now));
            }
        }

        LauncherEvent::CastExport(ExportProgress {
            instance_id: self.instance_id.clone(),
            stage,
            done,
            total,
        })
        .emit(&self.app);
    }

    fn throttled(&self, stage: ExportStage, done: u64, total: u64) {
        let due = lock(&self.last).is_none_or(|last| last.elapsed() >= PROGRESS_EVERY);
        let changed = lock(&self.stages)
            .last()
            .is_none_or(|(last, _)| *last != stage);

        if due || changed || (total > 0 && done >= total) {
            self.send(stage, done, total);
        }
    }

    fn hashed(&self, done: u64, total: u64) {
        match done >= total {
            true => self.send(ExportStage::Identifying, 0, 0),
            false => self.throttled(ExportStage::Hashing, done, total),
        }
    }

    fn timings(&self) -> String {
        let stages = lock(&self.stages);
        let now = Instant::now();

        stages
            .iter()
            .enumerate()
            .map(|(index, (stage, started))| {
                let until = stages.get(index + 1).map_or(now, |(_, next)| *next);
                format!("{stage:?} {:.1}s", (until - *started).as_secs_f64())
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
