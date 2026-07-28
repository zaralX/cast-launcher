use std::collections::HashMap;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::RwLock;

use cast_core::error::{CommandError, CommandResult};
use cast_core::fs_util::ensure_dir;
use cast_core::instance::{Instance, LoaderType};
use cast_core::java::{self, ResolveOptions};
use cast_core::meta::{self, Resolver};
use cast_core::mojang::profile::{resolve_libraries, ResolvedLibrary};
use cast_core::mojang::rules::RuntimeContext;
use cast_core::mojang::version::{AssetIndex, VersionPackage};
use cast_core::net::download::{DownloadOptions, DownloadTask};
use cast_core::paths::LauncherPaths;

use crate::events::{EmitExt, LauncherEvent};
use crate::state::AppState;

mod modpack;

pub use cast_core::install::phases::{self, job_id, job_prefix};
pub use cast_core::install::progress::{InstallSnapshot, ProgressReporter, Stage};

#[derive(Default)]
pub struct InstallRegistry {
    jobs: RwLock<HashMap<String, Arc<ProgressReporter>>>,
}

impl InstallRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn snapshots(&self) -> Vec<InstallSnapshot> {
        self.jobs.read().await.values().map(|job| job.snapshot()).collect()
    }

    pub async fn snapshot(&self, instance_id: &str) -> Option<InstallSnapshot> {
        self.jobs.read().await.get(instance_id).map(|job| job.snapshot())
    }

    pub async fn cancel(&self, instance_id: &str) {
        if let Some(job) = self.jobs.read().await.get(instance_id) {
            job.request_cancel();
        }
    }

    async fn register(&self, reporter: Arc<ProgressReporter>) {
        self.jobs
            .write()
            .await
            .insert(reporter.instance_id().to_string(), reporter);
    }

    async fn unregister(&self, instance_id: &str) {
        self.jobs.write().await.remove(instance_id);
    }
}

pub async fn start(
    app: AppHandle,
    state: Arc<AppState>,
    instance_id: String,
) -> CommandResult<InstallSnapshot> {
    if let Some(existing) = state.installs.snapshot(&instance_id).await {
        return Ok(existing);
    }

    let instance = state.instances.get(&instance_id).await?;

    let reporter = Arc::new(ProgressReporter::new(
        install_publisher(app.clone()),
        instance.id.clone(),
        instance.name.clone(),
        phases::for_install(instance.loader, instance.pack.is_some()),
    ));

    state.installs.register(Arc::clone(&reporter)).await;

    let snapshot = reporter.snapshot();

    tokio::spawn(async move {
        let outcome = run(&state, &instance, &reporter).await;
        complete(&app, &state, &instance, &reporter, outcome).await;
    });

    Ok(snapshot)
}

async fn complete(
    app: &AppHandle,
    state: &Arc<AppState>,
    instance: &Instance,
    reporter: &Arc<ProgressReporter>,
    outcome: CommandResult<()>,
) {
    match outcome {
        Ok(()) => {
            let paths = state.paths().await;

            if let Err(error) = state.instances.mark_installed(&paths, &instance.id).await {
                reporter.fail(Stage::Failed, error.message.clone());
                eprintln!("Установка завершена, но флаг не сохранился: {error}");
            } else {
                reporter.finish();
                LauncherEvent::Instances {
                    instances: state.instances.all().await,
                }
                .emit(app);
            }
        }
        Err(error) if error.is_aborted() || reporter.is_cancelled() => {
            reporter.fail(Stage::Aborted, "Установка прервана".into());
        }
        Err(error) => {
            eprintln!("Установка «{}» не удалась: {error}", instance.name);
            reporter.fail(Stage::Failed, error.message.clone());
        }
    }

    state.installs.unregister(&instance.id).await;
}

async fn run(
    state: &Arc<AppState>,
    instance: &Instance,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    let paths = state.paths().await;

    reporter.set_stage(Stage::Prepare);
    reporter.set_message("Подготовка");
    prepare_dirs(&paths, instance).await?;

    // Точные версии игры и загрузчика лежат в манифесте пака, поэтому сначала
    // забираем его и подтягиваем сборку под него.
    let (instance, modpack) = match &instance.pack {
        Some(pack) => {
            let modpack = modpack::prepare(state, &paths, instance, pack, reporter).await?;
            check_cancelled(reporter)?;

            let synced = modpack::sync_instance(state, &paths, instance, modpack.index()).await?;

            (synced, Some(modpack))
        }
        None => (instance.clone(), None),
    };

    let instance = &instance;

    let resolver = Resolver::new(&paths, &state.meta);
    let base = resolver.base_package(instance).await?;
    check_cancelled(reporter)?;

    let ctx = ensure_java(state, &paths, instance, &base, reporter).await?;
    check_cancelled(reporter)?;

    reporter.set_stage(Stage::Download);

    if instance.loader != LoaderType::Forge {
        download_client(state, &paths, instance, &base, reporter).await?;
        check_cancelled(reporter)?;
    }

    download_libraries(state, &paths, instance, &base, &ctx, reporter).await?;
    check_cancelled(reporter)?;

    download_assets(state, &paths, instance, &base, reporter).await?;
    check_cancelled(reporter)?;

    match instance.loader {
        LoaderType::Vanilla => {}
        LoaderType::Fabric => {
            install_fabric(state, &paths, instance, &resolver, reporter).await?;
        }
        LoaderType::Forge => {
            install_forge(state, &paths, instance, &base, reporter).await?;
        }
    }

    check_cancelled(reporter)?;

    if let Some(modpack) = &modpack {
        modpack::apply(state, &paths, instance, modpack, reporter).await?;
    }

    reporter.set_stage(Stage::Finalize);

    Ok(())
}

async fn prepare_dirs(paths: &LauncherPaths, instance: &Instance) -> CommandResult<()> {
    for dir in [
        paths.libraries(),
        paths.assets(),
        paths.asset_indexes(),
        paths.cache(),
        paths.instance(&instance.id).minecraft(),
    ] {
        ensure_dir(&dir).await?;
    }

    Ok(())
}

async fn ensure_java(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    base: &VersionPackage,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<RuntimeContext> {
    reporter.begin_phase("java", "Проверка Java");

    let requirement = cast_core::mojang::profile::JavaRequirement::from_package(base);
    let config = instance.effective_config(&state.config().await);

    let java = java::resolve(
        &state.java,
        &state.downloads,
        &state.meta,
        &config,
        paths.java_runtimes(),
        &requirement,
        ResolveOptions {
            allow_download: true,
            job_id: &job_id(&instance.id, "java"),
            on_progress: Some(download_reporter(reporter)),
        },
    )
    .await?;

    reporter.set_fraction(1.0);

    Ok(java.runtime_context())
}

async fn download_client(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    base: &VersionPackage,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    reporter.begin_phase("client", "Клиент Minecraft");

    let client = base
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.client.as_ref())
        .ok_or_else(|| CommandError::manifest("В манифесте версии нет ссылки на client.jar"))?;

    let task = DownloadTask::verified(
        client.url.clone(),
        paths.instance(&instance.id).client_jar(),
        client.size,
        client.sha1.clone(),
    );

    download(state, instance, "client", vec![task], reporter).await
}

async fn download_libraries(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    base: &VersionPackage,
    ctx: &RuntimeContext,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    reporter.begin_phase("libraries", "Библиотеки");

    let libraries = resolve_libraries(&base.libraries, ctx);
    let tasks = library_tasks(paths, &libraries);

    download(state, instance, "libraries", tasks, reporter).await
}

fn library_tasks(paths: &LauncherPaths, libraries: &[ResolvedLibrary]) -> Vec<DownloadTask> {
    libraries
        .iter()
        .flat_map(ResolvedLibrary::artifacts)
        .filter_map(|artifact| {
            let url = artifact.url.as_ref()?;

            Some(DownloadTask::verified(
                url.clone(),
                paths.library(&artifact.path),
                artifact.size,
                artifact.sha1.clone(),
            ))
        })
        .collect()
}

async fn download_assets(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    base: &VersionPackage,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    reporter.begin_phase("assets", "Ресурсы игры");

    let Some(asset_index) = &base.asset_index else {
        return Err(CommandError::manifest("В манифесте версии нет индекса ассетов"));
    };

    let index_path = paths.asset_index(&asset_index.id);
    let index_task = DownloadTask::verified(
        asset_index.url.clone(),
        index_path.clone(),
        asset_index.size,
        asset_index.sha1.clone(),
    );

    state
        .downloads
        .run(
            job_id(&instance.id, "asset-index"),
            vec![index_task],
            DownloadOptions { deep_verify: true },
            None,
        )
        .await?;

    let index: AssetIndex = cast_core::fs_util::read_json(&index_path).await?;

    let tasks = index
        .objects
        .values()
        .map(|object| {
            DownloadTask::verified(
                object.url(),
                paths.asset_object(&object.hash),
                object.size,
                Some(object.hash.clone()),
            )
        })
        .collect();

    download(state, instance, "assets", tasks, reporter).await
}

async fn install_fabric(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    resolver: &Resolver<'_>,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    reporter.begin_phase("fabric", "Библиотеки Fabric");

    let loader = resolver.fabric_loader(instance).await?;
    let libraries = meta::fabric::libraries(&loader)?;
    let tasks = library_tasks(paths, &libraries);

    download(state, instance, "fabric", tasks, reporter).await
}

async fn install_forge(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    base: &VersionPackage,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    let forge_version = instance.require_loader_version()?.to_string();
    let cache = paths.forge_cache(&forge_version);

    if cache.is_installed() {
        return Ok(());
    }

    reporter.begin_phase("forge-installer", "Установщик Forge");

    let installer_jar = cache.installer_jar();

    if !installer_jar.is_file() {
        let task = DownloadTask::new(meta::forge::installer_url(&forge_version), installer_jar.clone());
        download(state, instance, "forge-installer", vec![task], reporter).await?;
    }

    reporter.set_stage(Stage::Install);
    reporter.begin_phase("forge-install", "Установщик Forge работает");

    let java = resolve_java_for_forge(state, paths, instance, base).await?;

    cast_core::install::forge::install(
        paths,
        &java,
        &forge_version,
        &installer_jar,
        |line| reporter.set_message(cast_core::install::forge::trim_log_line(line)),
    )
    .await?;

    reporter.set_fraction(1.0);

    Ok(())
}

async fn resolve_java_for_forge(
    state: &Arc<AppState>,
    paths: &LauncherPaths,
    instance: &Instance,
    base: &VersionPackage,
) -> CommandResult<String> {
    let config = instance.effective_config(&state.config().await);
    let requirement = cast_core::mojang::profile::JavaRequirement::from_package(base);

    let java = java::resolve(
        &state.java,
        &state.downloads,
        &state.meta,
        &config,
        paths.java_runtimes(),
        &requirement,
        ResolveOptions::default(),
    )
    .await?;

    Ok(java.path)
}

async fn download(
    state: &Arc<AppState>,
    instance: &Instance,
    phase: &str,
    tasks: Vec<DownloadTask>,
    reporter: &Arc<ProgressReporter>,
) -> CommandResult<()> {
    if tasks.is_empty() {
        reporter.set_fraction(1.0);
        return Ok(());
    }

    state
        .downloads
        .run(
            job_id(&instance.id, phase),
            tasks,
            DownloadOptions::default(),
            Some(download_reporter(reporter)),
        )
        .await?;

    reporter.set_fraction(1.0);

    Ok(())
}

fn install_publisher(app: AppHandle) -> cast_core::install::progress::Publisher {
    Arc::new(move |snapshot| LauncherEvent::Install(snapshot).emit(&app))
}

fn download_reporter(
    reporter: &Arc<ProgressReporter>,
) -> cast_core::net::download::ProgressSink {
    let reporter = Arc::clone(reporter);
    Box::new(move |snapshot| reporter.apply_download(snapshot))
}

fn check_cancelled(reporter: &Arc<ProgressReporter>) -> CommandResult<()> {
    if reporter.is_cancelled() {
        return Err(CommandError::aborted("Установка прервана"));
    }
    Ok(())
}


