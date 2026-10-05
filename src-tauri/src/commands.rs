use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tokio::task::JoinSet;

use cast_core::account::{Account, AccountConfig};
use cast_core::assets::{self, ItemCategories};
use cast_core::config::AppConfig;
use cast_core::error::{CommandError, CommandResult};
use cast_core::icons::{self, IconFile};
use cast_core::import::{ImportProgress, ImportReport, LauncherKind, ScannedInstance};
use cast_core::install::pack_files::{self, PackFiles};
use cast_core::instance::{Instance, InstanceUpdate, PackProvider};
use cast_core::java::detect::JavaRuntime;
use cast_core::logs::{self, LogFile};
use cast_core::meta::{neoforge, vanilla};
use cast_core::mods::catalog::{CatalogMatch, CatalogVersion};
use cast_core::mods::install::{InstallPlan, ModSearch};
use cast_core::mods::updates::ModUpdate;
use cast_core::mods::{self, ModFile};
use cast_core::mojang::version::VersionManifest;
use cast_core::packs::{self, PackPage};
use cast_core::paths::PathsSnapshot;
use cast_core::skins::{self, AccountLook, SkinEntry, SkinLibrary, SkinVariant};

use crate::events::{EmitExt, LauncherEvent};
use crate::import;
use crate::install::{self, InstallSnapshot};
use crate::state::AppState;
use crate::telemetry::{self, Event};
use cast_core::launch::game::RunningGame;

type Ctx<'a> = State<'a, Arc<AppState>>;

const MAX_PLANS: usize = 32;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub config: AppConfig,
    pub paths: PathsSnapshot,
    pub accounts: AccountConfig,
    pub instances: Vec<Instance>,
    pub installs: Vec<InstallSnapshot>,
    pub running: Vec<RunningGame>,
    pub import: Option<ImportProgress>,
}

#[tauri::command]
pub async fn bootstrap(state: Ctx<'_>) -> CommandResult<Bootstrap> {
    let paths = state.paths().await;

    Ok(Bootstrap {
        config: state.config().await,
        paths: PathsSnapshot::from(&paths),
        accounts: state.accounts.config().await,
        instances: state.instances.all().await,
        installs: state.installs.snapshots().await,
        running: state.processes.running().await,
        import: state.imports.progress(),
    })
}

#[tauri::command]
pub async fn get_config(state: Ctx<'_>) -> CommandResult<AppConfig> {
    Ok(state.config().await)
}

#[tauri::command]
pub async fn update_config(
    app: AppHandle,
    state: Ctx<'_>,
    config: AppConfig,
) -> CommandResult<AppConfig> {
    let before = state.config().await;
    let relocated = state.update_config(config).await?;
    let updated = state.config().await;

    if relocated {
        LauncherEvent::Instances {
            instances: state.instances.all().await,
        }
        .emit(&app);
    }

    telemetry::settings_changed(&app, &before, &updated);
    telemetry::set_enabled(updated.launcher.telemetry);

    Ok(updated)
}

#[tauri::command]
pub async fn get_paths(state: Ctx<'_>) -> CommandResult<PathsSnapshot> {
    Ok(PathsSnapshot::from(&state.paths().await))
}

#[tauri::command]
pub async fn open_path(app: AppHandle, path: String) -> CommandResult<()> {
    open(&app, Path::new(&path))
}

#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> CommandResult<()> {
    let parsed = url::Url::parse(url.trim()).map_err(|e| {
        CommandError::unknown("error.reason.links.invalid")
            .param("url", &url)
            .with_details(e.to_string())
    })?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(CommandError::unknown("error.reason.links.unsupported").param("url", &url));
    }

    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| {
            CommandError::unknown("error.reason.links.open_failed")
                .param("url", &url)
                .with_details(e.to_string())
        })
}

fn open(app: &AppHandle, path: &Path) -> CommandResult<()> {
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| {
            CommandError::fs("error.reason.fs.open")
                .param("path", path.display())
                .with_details(e.to_string())
        })
}

#[tauri::command]
pub async fn list_instances(state: Ctx<'_>) -> CommandResult<Vec<Instance>> {
    Ok(state.instances.all().await)
}

#[tauri::command]
pub async fn reload_instances(app: AppHandle, state: Ctx<'_>) -> CommandResult<Vec<Instance>> {
    let paths = state.paths().await;
    let instances = state.instances.reload(&paths).await?;

    LauncherEvent::Instances {
        instances: instances.clone(),
    }
    .emit(&app);

    Ok(instances)
}

#[tauri::command]
pub async fn create_instance(
    app: AppHandle,
    state: Ctx<'_>,
    instance: Instance,
) -> CommandResult<Instance> {
    let paths = state.paths().await;
    let created = state.instances.create(&paths, instance).await?;

    telemetry::track(&app, Event::new("instance_created").instance(&created));

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(created)
}

#[tauri::command]
pub async fn delete_instance(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<()> {
    if state.processes.is_running(&instance_id).await {
        return Err(CommandError::conflict(
            "error.reason.instance.close_game_first",
        ));
    }

    state.installs.cancel(&instance_id).await;

    let removed = state.instances.get(&instance_id).await.ok();

    let paths = state.paths().await;
    state.instances.remove(&paths, &instance_id).await?;

    if let Some(removed) = &removed {
        telemetry::track(
            &app,
            Event::new("instance_deleted")
                .instance(removed)
                .flag("installed", removed.installed)
                .num(
                    "playtime_min",
                    telemetry::minutes(removed.playtime.total_seconds),
                ),
        );
    }

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(())
}

#[tauri::command]
pub async fn update_instance(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
    update: InstanceUpdate,
) -> CommandResult<Instance> {
    let paths = state.paths().await;
    let update = update.normalized(&paths.icons())?;

    let updated = state
        .instances
        .update(&paths, &instance_id, move |instance| update.apply(instance))
        .await?;

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(updated)
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceDir {
    #[default]
    Root,
    Minecraft,
    Mods,
    Logs,
}

#[tauri::command]
pub async fn open_instance_dir(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
    target: InstanceDir,
) -> CommandResult<()> {
    let paths = state.paths().await;
    let instance = paths.instance(&instance_id);

    let dir = match target {
        InstanceDir::Root => instance.root().to_path_buf(),
        InstanceDir::Minecraft => instance.minecraft(),
        InstanceDir::Mods => instance.mods(),
        InstanceDir::Logs => paths.instance_logs(&instance_id),
    };

    cast_core::fs_util::ensure_dir(&dir).await?;

    open(&app, &dir)
}

#[tauri::command]
pub async fn list_instance_logs(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<Vec<LogFile>> {
    let paths = state.paths().await;
    logs::list(&paths.instance_logs(&instance_id)).await
}

#[tauri::command]
pub async fn read_instance_log(
    state: Ctx<'_>,
    instance_id: String,
    name: String,
) -> CommandResult<String> {
    let paths = state.paths().await;
    let path = logs::resolve(&paths.instance_logs(&instance_id), &name)?;

    logs::read_tail(&path, logs::TAIL_LIMIT).await
}

#[tauri::command]
pub async fn delete_instance_log(
    state: Ctx<'_>,
    instance_id: String,
    name: String,
) -> CommandResult<Vec<LogFile>> {
    let paths = state.paths().await;
    let dir = paths.instance_logs(&instance_id);

    logs::remove(&logs::resolve(&dir, &name)?).await?;

    logs::list(&dir).await
}

#[tauri::command]
pub async fn list_instance_mods(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<Vec<ModFile>> {
    let scan = mods_scan(&state, &instance_id).await?;

    mods::list(&scan, false).await
}

#[tauri::command]
pub async fn refresh_instance_mods(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<Vec<ModFile>> {
    let scan = mods_scan(&state, &instance_id).await?;

    mods::icon::prune(&scan.icons).await;

    mods::list(&scan, true).await
}

#[tauri::command]
pub async fn read_mod_icon(state: Ctx<'_>, key: String) -> CommandResult<String> {
    let paths = state.paths().await;

    mods::icon::data_url(&paths.mod_icons(), &key).await
}

#[tauri::command]
pub async fn set_mod_enabled(
    state: Ctx<'_>,
    instance_id: String,
    path: String,
    enabled: bool,
) -> CommandResult<Vec<ModFile>> {
    let lock = state.mods.of(&instance_id).await;
    let _guard = lock.lock().await;

    let scan = mods_scan(&state, &instance_id).await?;

    mods::manage::set_enabled(&scan, &path, enabled).await?;

    mods::list(&scan, false).await
}

#[tauri::command]
pub async fn delete_mods(
    state: Ctx<'_>,
    instance_id: String,
    paths: Vec<String>,
) -> CommandResult<Vec<ModFile>> {
    let lock = state.mods.of(&instance_id).await;
    let _guard = lock.lock().await;

    let scan = mods_scan(&state, &instance_id).await?;

    let removed = mods::manage::remove(&scan, &paths).await;

    let dirs = state.paths().await.instance(&instance_id);

    match pack_files::forget_deleted(&dirs.pack_files(), &dirs.minecraft(), &paths).await {
        Ok(0) => {}
        Ok(count) => log::info!("Instance '{instance_id}': {count} deleted pack files forgotten"),
        Err(error) => {
            log::warn!("Failed to forget deleted pack files of instance '{instance_id}': {error}")
        }
    }

    removed?;

    mods::list(&scan, false).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedMods {
    pub mods: Vec<ModFile>,
    pub report: mods::manage::Installed,
}

#[tauri::command]
pub async fn add_mods(
    state: Ctx<'_>,
    instance_id: String,
    paths: Vec<String>,
) -> CommandResult<AddedMods> {
    let lock = state.mods.of(&instance_id).await;
    let _guard = lock.lock().await;

    let scan = mods_scan(&state, &instance_id).await?;
    let sources: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    let report = mods::manage::install(&scan, &sources).await?;

    Ok(AddedMods {
        mods: mods::list(&scan, false).await?,
        report,
    })
}

#[tauri::command]
pub async fn identify_instance_mods(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<BTreeMap<String, CatalogMatch>> {
    let scan = mods_scan(&state, &instance_id).await?;
    let paths = state.paths().await;

    let mods = mods::list(&scan, false).await?;

    mods::catalog::identify(&scan, &mods, &paths.mod_catalog()).await
}

#[tauri::command]
pub async fn check_mod_updates(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<Vec<ModUpdate>> {
    let instance = state.instances.get(&instance_id).await?;
    let scan = mods_scan(&state, &instance_id).await?;
    let paths = state.paths().await;

    let mods = mods::list(&scan, false).await?;
    let matches = mods::catalog::identify(&scan, &mods, &paths.mod_catalog()).await?;

    let found = mods::updates::check(
        &mods,
        &matches,
        instance.loader,
        &instance.minecraft_version,
    )
    .await;

    state
        .mod_updates
        .write()
        .await
        .insert(instance_id, found.clone());

    Ok(found)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedMods {
    pub mods: Vec<ModFile>,
    pub report: mods::updates::Updated,
}

#[tauri::command]
pub async fn update_mods(
    state: Ctx<'_>,
    instance_id: String,
    paths: Vec<String>,
) -> CommandResult<UpdatedMods> {
    let lock = state.mods.of(&instance_id).await;
    let _guard = lock.lock().await;

    let wanted: Vec<ModUpdate> = state
        .mod_updates
        .read()
        .await
        .get(&instance_id)
        .map(|found| {
            found
                .iter()
                .filter(|update| paths.contains(&update.path))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    if wanted.is_empty() {
        return Err(CommandError::conflict("error.reason.mods.updates_stale"));
    }

    let scan = mods_scan(&state, &instance_id).await?;
    let launcher = state.paths().await;

    let report =
        mods::updates::apply(&scan, &state.downloads, &wanted, &launcher.mod_catalog()).await?;

    if let Some(found) = state.mod_updates.write().await.get_mut(&instance_id) {
        found.retain(|update| !paths.contains(&update.path));
    }

    Ok(UpdatedMods {
        mods: mods::list(&scan, false).await?,
        report,
    })
}

#[tauri::command]
pub async fn search_mods(
    state: Ctx<'_>,
    instance_id: String,
    query: ModSearch,
) -> CommandResult<PackPage> {
    mods::install::search(&with_instance(&state, &instance_id, query).await?).await
}

#[tauri::command]
pub async fn mod_versions(
    state: Ctx<'_>,
    instance_id: String,
    provider: PackProvider,
    project_id: String,
) -> CommandResult<Vec<CatalogVersion>> {
    let instance = state.instances.get(&instance_id).await?;

    mods::install::versions(
        provider,
        &project_id,
        instance.loader,
        &instance.minecraft_version,
    )
    .await
}

#[tauri::command]
pub async fn plan_mod_install(
    state: Ctx<'_>,
    instance_id: String,
    provider: PackProvider,
    project_id: String,
    version_id: String,
) -> CommandResult<InstallPlan> {
    let instance = state.instances.get(&instance_id).await?;
    let installed = installed_projects(&state, &instance_id).await?;

    let plan = mods::install::plan(mods::install::PlanRequest {
        provider,
        project_id: &project_id,
        version_id: &version_id,
        loader: instance.loader,
        game_version: &instance.minecraft_version,
        installed: &installed,
    })
    .await?;

    let mut plans = state.mod_plans.write().await;

    if plans.len() >= MAX_PLANS {
        plans.clear();
    }

    plans.insert(plan.id.clone(), (instance_id, plan.clone()));

    Ok(plan)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMods {
    pub mods: Vec<ModFile>,
    pub report: mods::install::InstallReport,
}

#[tauri::command]
pub async fn install_mod(
    state: Ctx<'_>,
    instance_id: String,
    plan_id: String,
    optional: Vec<String>,
) -> CommandResult<InstalledMods> {
    let lock = state.mods.of(&instance_id).await;
    let _guard = lock.lock().await;

    let plan = state
        .mod_plans
        .read()
        .await
        .get(&plan_id)
        .filter(|(planned_for, _)| planned_for == &instance_id)
        .map(|(_, plan)| plan.clone())
        .ok_or_else(|| CommandError::conflict("error.reason.mods.plan_stale"))?;

    let scan = mods_scan(&state, &instance_id).await?;
    let paths = state.paths().await;

    let report = mods::install::apply(
        &scan,
        &state.downloads,
        &plan,
        &optional,
        &paths.mod_catalog(),
    )
    .await?;

    state.mod_plans.write().await.remove(&plan_id);

    Ok(InstalledMods {
        mods: mods::list(&scan, false).await?,
        report,
    })
}

async fn with_instance(
    state: &Ctx<'_>,
    instance_id: &str,
    query: ModSearch,
) -> CommandResult<ModSearch> {
    let instance = state.instances.get(instance_id).await?;

    Ok(ModSearch {
        loader: instance.loader,
        game_version: instance.minecraft_version,
        ..query
    })
}

async fn installed_projects(state: &Ctx<'_>, instance_id: &str) -> CommandResult<BTreeSet<String>> {
    let scan = mods_scan(state, instance_id).await?;
    let paths = state.paths().await;

    let mods = mods::list(&scan, false).await?;
    let matches = mods::catalog::identify(&scan, &mods, &paths.mod_catalog()).await?;

    Ok(matches
        .values()
        .map(|matched| matched.project_id.clone())
        .collect())
}

/// Native dialog wording, translated by the frontend.
#[derive(Debug, Deserialize)]
pub struct DialogText {
    pub title: String,
    #[serde(default)]
    pub filter: String,
}

#[tauri::command]
pub async fn pick_mod_files(app: AppHandle, dialog: DialogText) -> CommandResult<Vec<String>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title(dialog.title)
        .add_filter(dialog.filter, &["jar", "zip", "litemod"])
        .pick_files(move |picked| {
            let _ = sender.send(picked);
        });

    let picked = receiver.await.ok().flatten().unwrap_or_default();

    Ok(picked
        .into_iter()
        .filter_map(|file| file.into_path().ok())
        .map(|path| path.display().to_string())
        .collect())
}

async fn mods_scan(state: &Ctx<'_>, instance_id: &str) -> CommandResult<mods::ModsScan> {
    let instance = state.instances.get(instance_id).await?;
    let paths = state.paths().await;
    let dirs = paths.instance(instance_id);

    Ok(mods::ModsScan {
        dir: dirs.mods(),
        index_file: dirs.mods_index(),
        icons: paths.mod_icons(),
        loader: mods::ModLoader::of(instance.loader),
        managed: managed_mods(&dirs).await,
    })
}

async fn managed_mods(dirs: &cast_core::paths::InstancePaths) -> BTreeSet<String> {
    let record = PackFiles::load(&dirs.pack_files()).await;

    record
        .paths
        .union(&record.extracted)
        .filter(|path| path.starts_with(&format!("{}/", mods::FOLDER)))
        .cloned()
        .collect()
}

#[tauri::command]
pub async fn list_icons(state: Ctx<'_>) -> CommandResult<Vec<IconFile>> {
    let paths = state.paths().await;
    icons::list(&paths.icons()).await
}

#[tauri::command]
pub async fn read_icon(state: Ctx<'_>, name: String) -> CommandResult<String> {
    let paths = state.paths().await;
    icons::data_url(&icons::resolve(&paths.icons(), &name)?).await
}

#[tauri::command]
pub async fn import_icon(
    app: AppHandle,
    state: Ctx<'_>,
    path: Option<String>,
    dialog: DialogText,
) -> CommandResult<Option<IconFile>> {
    let source = match path {
        Some(path) => Some(PathBuf::from(path)),
        None => pick_image(&app, dialog).await,
    };

    let Some(source) = source else {
        return Ok(None);
    };

    let paths = state.paths().await;

    icons::import(&paths.icons(), &source).await.map(Some)
}

async fn pick_image(app: &AppHandle, dialog: DialogText) -> Option<PathBuf> {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title(dialog.title)
        .add_filter(dialog.filter, &icons::extensions())
        .pick_file(move |picked| {
            let _ = sender.send(picked);
        });

    receiver
        .await
        .ok()
        .flatten()
        .and_then(|picked| picked.into_path().ok())
}

#[tauri::command]
pub async fn delete_icon(state: Ctx<'_>, name: String) -> CommandResult<Vec<IconFile>> {
    let paths = state.paths().await;

    icons::remove(&paths.icons(), &name).await?;
    icons::list(&paths.icons()).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCatalog {
    pub categories: ItemCategories,
    pub names: BTreeMap<String, String>,
}

#[tauri::command]
pub async fn list_item_icons(state: Ctx<'_>) -> CommandResult<ItemCatalog> {
    let categories = assets::item_categories(&state.meta).await?;
    let language = state.config().await.launcher.language;

    let names = assets::item_names(&state.meta, &language)
        .await
        .unwrap_or_default();

    Ok(ItemCatalog { categories, names })
}

const CATALOG_CONCURRENCY: usize = 8;

#[tauri::command]
pub async fn item_icons(
    state: Ctx<'_>,
    items: Vec<String>,
) -> CommandResult<BTreeMap<String, String>> {
    let state = state.inner().clone();
    let mut queue = items.into_iter().filter(|item| assets::is_item_id(item));

    let mut tasks = JoinSet::new();
    let mut fetched = BTreeMap::new();

    for _ in 0..CATALOG_CONCURRENCY {
        match queue.next() {
            Some(item) => fetch_item_icon(&mut tasks, &state, item),
            None => break,
        }
    }

    while let Some(joined) = tasks.join_next().await {
        if let Ok((item, Some(url))) = joined {
            fetched.insert(item, url);
        }

        if let Some(item) = queue.next() {
            fetch_item_icon(&mut tasks, &state, item);
        }
    }

    Ok(fetched)
}

fn fetch_item_icon(
    tasks: &mut JoinSet<(String, Option<String>)>,
    state: &Arc<AppState>,
    item: String,
) {
    let state = Arc::clone(state);

    tasks.spawn(async move {
        let url = assets::item_icon(&state.meta, &item)
            .await
            .map(|bytes| icons::to_data_url("image/webp", &bytes))
            .ok();

        (item, url)
    });
}

#[tauri::command]
pub async fn save_item_icon(state: Ctx<'_>, item: String) -> CommandResult<IconFile> {
    let bytes = assets::item_icon(&state.meta, &item).await?;
    let paths = state.paths().await;

    icons::save_once(&paths.icons(), &assets::item_icon_file(&item), &bytes).await
}

#[tauri::command]
pub async fn install_instance(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<InstallSnapshot> {
    install::start(app, state.inner().clone(), instance_id).await
}

#[tauri::command]
pub async fn cancel_install(state: Ctx<'_>, instance_id: String) -> CommandResult<()> {
    state.installs.cancel(&instance_id).await;
    state
        .downloads
        .cancel_prefix(&install::job_prefix(&instance_id));
    state.blocked.resume(&instance_id).await;

    Ok(())
}

#[tauri::command]
pub async fn awaited_files(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<Vec<packs::BlockedFile>> {
    Ok(state.blocked.files(&instance_id).await)
}

#[tauri::command]
pub async fn downloads_dir() -> CommandResult<Option<String>> {
    Ok(install::blocked::default_downloads_dir().map(|dir| dir.display().to_string()))
}

#[tauri::command]
pub async fn scan_for_files(
    state: Ctx<'_>,
    instance_id: String,
    folder: String,
) -> CommandResult<Vec<packs::BlockedFile>> {
    let folder = folder.trim();

    if folder.is_empty() {
        return Err(CommandError::invalid_input(
            "error.reason.files.folder_required",
        ));
    }

    state.blocked.scan(&instance_id, Path::new(folder)).await
}

#[tauri::command]
pub async fn rescan_files(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<Vec<packs::BlockedFile>> {
    state.blocked.rescan(&instance_id).await;

    Ok(state.blocked.files(&instance_id).await)
}

#[tauri::command]
pub async fn pick_folder(
    app: AppHandle,
    title: String,
    directory: Option<String>,
) -> CommandResult<Option<String>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    let mut dialog = app.dialog().file().set_title(title);

    let start = directory
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .or_else(install::blocked::default_downloads_dir);

    if let Some(start) = start {
        dialog = dialog.set_directory(start);
    }

    dialog.pick_folder(move |picked| {
        let _ = sender.send(picked);
    });

    let picked = receiver
        .await
        .ok()
        .flatten()
        .and_then(|picked| picked.into_path().ok());

    Ok(picked.map(|path| path.display().to_string()))
}

#[tauri::command]
pub async fn resume_install(state: Ctx<'_>, instance_id: String) -> CommandResult<()> {
    state.blocked.resume(&instance_id).await;

    Ok(())
}

#[tauri::command]
pub async fn list_installs(state: Ctx<'_>) -> CommandResult<Vec<InstallSnapshot>> {
    Ok(state.installs.snapshots().await)
}

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<RunningGame> {
    crate::launch::launch(app, state.inner().clone(), &instance_id).await
}

#[tauri::command]
pub async fn play_instance(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<crate::play::PlayOutcome> {
    crate::play::play(app, state.inner().clone(), &instance_id).await
}

#[tauri::command]
pub async fn list_running(state: Ctx<'_>) -> CommandResult<Vec<RunningGame>> {
    Ok(state.processes.running().await)
}

#[tauri::command]
pub async fn stop_instance(state: Ctx<'_>, instance_id: String) -> CommandResult<usize> {
    Ok(state.processes.kill_instance(&instance_id).await)
}

#[tauri::command]
pub async fn list_java(state: Ctx<'_>, force: bool) -> CommandResult<Vec<JavaRuntime>> {
    let paths = state.paths().await;
    state.java.list(paths.java_runtimes(), force).await
}

#[tauri::command]
pub async fn probe_java(path: String) -> CommandResult<Option<JavaRuntime>> {
    cast_core::java::detect::probe(path).await
}

#[tauri::command]
pub async fn list_accounts(state: Ctx<'_>) -> CommandResult<AccountConfig> {
    Ok(state.accounts.config().await)
}

#[tauri::command]
pub async fn select_account(state: Ctx<'_>, index: usize) -> CommandResult<AccountConfig> {
    state.accounts.select(index).await
}

#[tauri::command]
pub async fn remove_account(
    app: AppHandle,
    state: Ctx<'_>,
    uuid: String,
) -> CommandResult<AccountConfig> {
    let removed = state.accounts.remove(&uuid).await?;

    telemetry::track(
        &app,
        Event::new("account_removed").num("left", removed.accounts.len() as f64),
    );

    Ok(removed)
}

#[tauri::command]
pub async fn add_offline_account(
    app: AppHandle,
    state: Ctx<'_>,
    name: String,
) -> CommandResult<AccountConfig> {
    let config = state.accounts.add_offline(&name).await?;

    telemetry::track(&app, Event::new("account_added").text("type", "offline"));

    Ok(config)
}

#[tauri::command]
pub async fn login_microsoft(
    app: AppHandle,
    state: Ctx<'_>,
    page: cast_core::account::oauth::LoginPage,
) -> CommandResult<Account> {
    let opener = app.clone();

    let account = cast_core::account::oauth::login(page, move |url| {
        opener.opener().open_url(url, None::<&str>).map_err(|e| {
            CommandError::auth("error.reason.account.browser_failed").with_details(e.to_string())
        })
    })
    .await
    .inspect_err(|error| telemetry::track(&app, Event::new("auth_failed").error(error)))?;

    state.accounts.upsert(account.clone()).await?;

    telemetry::track(&app, Event::new("account_added").text("type", "microsoft"));

    Ok(account)
}

#[tauri::command]
pub async fn refresh_account(
    app: AppHandle,
    state: Ctx<'_>,
    uuid: String,
) -> CommandResult<Account> {
    state.accounts.refresh(&uuid).await.inspect_err(|error| {
        telemetry::track(&app, Event::new("account_refresh_failed").error(error))
    })
}

#[tauri::command]
pub async fn skin_library(state: Ctx<'_>) -> CommandResult<SkinLibrary> {
    let paths = state.paths().await;
    Ok(skins::library::load(&paths.skins()).await)
}

#[tauri::command]
pub async fn skin_texture(state: Ctx<'_>, texture: String) -> CommandResult<String> {
    let paths = state.paths().await;
    skins::library::data_url(&paths.skins(), &texture).await
}

#[tauri::command]
pub async fn import_skin(
    app: AppHandle,
    state: Ctx<'_>,
    path: Option<String>,
    dialog: DialogText,
) -> CommandResult<Option<SkinEntry>> {
    let source = match path {
        Some(path) => Some(PathBuf::from(path)),
        None => pick_skin_file(&app, dialog).await,
    };

    let Some(source) = source else {
        return Ok(None);
    };

    let bytes = tokio::fs::read(&source)
        .await
        .map_err(|e| CommandError::io("error.reason.fs.read_file", &source, e))?;

    let name = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();

    let paths = state.paths().await;

    skins::library::add(
        &paths.skins(),
        &name,
        &bytes,
        skins::SkinSource::Local,
        None,
    )
    .await
    .map(Some)
}

async fn pick_skin_file(app: &AppHandle, dialog: DialogText) -> Option<PathBuf> {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title(dialog.title)
        .add_filter(dialog.filter, &["png"])
        .pick_file(move |picked| {
            let _ = sender.send(picked);
        });

    receiver
        .await
        .ok()
        .flatten()
        .and_then(|picked| picked.into_path().ok())
}

#[tauri::command]
pub async fn import_player_skin(state: Ctx<'_>, name: String) -> CommandResult<SkinEntry> {
    let paths = state.paths().await;
    skins::import_player(&paths.skins(), &name).await
}

#[tauri::command]
pub async fn rename_skin(state: Ctx<'_>, id: String, name: String) -> CommandResult<SkinLibrary> {
    let paths = state.paths().await;
    skins::library::rename(&paths.skins(), &id, &name).await
}

#[tauri::command]
pub async fn set_skin_variant(
    state: Ctx<'_>,
    id: String,
    variant: SkinVariant,
) -> CommandResult<SkinLibrary> {
    let paths = state.paths().await;
    skins::library::set_variant(&paths.skins(), &id, variant).await
}

#[tauri::command]
pub async fn delete_skin(state: Ctx<'_>, id: String) -> CommandResult<SkinLibrary> {
    let paths = state.paths().await;
    skins::library::remove(&paths.skins(), &id).await
}

#[tauri::command]
pub async fn set_skin_cape(
    state: Ctx<'_>,
    id: String,
    cape_id: Option<String>,
) -> CommandResult<SkinLibrary> {
    let paths = state.paths().await;
    skins::library::set_cape(&paths.skins(), &id, cape_id).await
}

#[tauri::command]
pub async fn duplicate_skin(
    state: Ctx<'_>,
    id: String,
    cape_id: Option<String>,
    name: String,
) -> CommandResult<SkinEntry> {
    let paths = state.paths().await;
    skins::library::duplicate(&paths.skins(), &id, cape_id, &name).await
}

#[tauri::command]
pub async fn account_look(
    state: Ctx<'_>,
    uuid: String,
    refresh: bool,
) -> CommandResult<AccountLook> {
    let paths = state.paths().await;
    skins::look(&state.accounts, &paths.skins(), &uuid, refresh).await
}

#[tauri::command]
pub async fn apply_skin(
    app: AppHandle,
    state: Ctx<'_>,
    uuid: String,
    id: String,
) -> CommandResult<AccountLook> {
    let paths = state.paths().await;
    let look = skins::apply_skin(&state.accounts, &paths.skins(), &uuid, &id).await;

    telemetry::track(
        &app,
        match &look {
            Ok(look) => Event::new("skin_applied").text("variant", look.variant.as_api()),
            Err(error) => Event::new("skin_apply_failed").error(error),
        },
    );

    look
}

#[tauri::command]
pub async fn reset_skin(state: Ctx<'_>, uuid: String) -> CommandResult<AccountLook> {
    let paths = state.paths().await;
    skins::reset_skin(&state.accounts, &paths.skins(), &uuid).await
}

#[tauri::command]
pub async fn apply_cape(
    app: AppHandle,
    state: Ctx<'_>,
    uuid: String,
    cape_id: Option<String>,
) -> CommandResult<AccountLook> {
    let paths = state.paths().await;
    let look = skins::apply_cape(&state.accounts, &paths.skins(), &uuid, cape_id.as_deref()).await;

    telemetry::track(
        &app,
        match &look {
            Ok(_) => Event::new("cape_applied")
                .text("cape", if cape_id.is_some() { "on" } else { "off" }),
            Err(error) => Event::new("cape_apply_failed").error(error),
        },
    );

    look
}

#[tauri::command]
pub async fn castpack_catalog(
    app: AppHandle,
    state: Ctx<'_>,
) -> CommandResult<cast_core::castpack::Catalog> {
    crate::castpack::catalog(&app, state.inner()).await
}

#[tauri::command]
pub async fn castpack_install(
    app: AppHandle,
    state: Ctx<'_>,
    pack_id: String,
) -> CommandResult<Instance> {
    crate::castpack::install_pack(app, state.inner().clone(), &pack_id).await
}

#[tauri::command]
pub async fn castpack_check_update(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<crate::play::CastPackUpdate> {
    crate::play::check_update(&app, state.inner(), &instance_id).await
}

#[tauri::command]
pub async fn castpack_set_autoupdate(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
    enabled: bool,
) -> CommandResult<Instance> {
    crate::castpack::set_autoupdate(&app, state.inner(), &instance_id, enabled).await
}

#[tauri::command]
pub async fn pack_providers() -> CommandResult<Vec<packs::ProviderInfo>> {
    Ok(packs::providers())
}

#[tauri::command]
pub async fn search_packs(
    app: AppHandle,
    query: packs::SearchQuery,
) -> CommandResult<packs::PackPage> {
    let page = packs::search(&query).await?;

    if query.offset == 0 {
        let filters = query.categories.len() + query.loaders.len() + query.game_versions.len();

        telemetry::track(
            &app,
            Event::new("pack_search")
                .text("provider", query.provider.key())
                .flag("has_query", !query.query.trim().is_empty())
                .num("filters", filters as f64)
                .num("results", page.total_hits)
                .maybe("sort", query.sort.as_deref())
                .maybe("environment", query.environment.as_deref()),
        );
    }

    Ok(page)
}

#[tauri::command]
pub async fn list_pack_versions(
    provider: PackProvider,
    project_id: String,
) -> CommandResult<Vec<packs::PackVersion>> {
    packs::versions(provider, &project_id).await
}

#[tauri::command]
pub async fn pack_filters(
    state: Ctx<'_>,
    provider: PackProvider,
) -> CommandResult<packs::PackFilters> {
    packs::filters(provider, &state.meta).await
}

#[tauri::command]
pub async fn set_instance_pack_version(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
    version_id: String,
) -> CommandResult<Instance> {
    if state.processes.is_running(&instance_id).await {
        return Err(CommandError::conflict(
            "error.reason.instance.close_game_first",
        ));
    }

    if state.installs.snapshot(&instance_id).await.is_some() {
        return Err(CommandError::conflict(
            "error.reason.instance.wait_for_install",
        ));
    }

    let instance = state.instances.get(&instance_id).await?;
    let current = packs::switch::switchable_pack(&instance)?;
    let version = packs::version(current.provider, &current.project_id, &version_id).await?;
    let (pack, loader, minecraft_version) = packs::switch::switch_to(current, version)?;

    let paths = state.paths().await;

    let updated = state
        .instances
        .update(&paths, &instance_id, move |instance| {
            instance.pack = Some(pack);
            instance.loader = loader;
            instance.minecraft_version = minecraft_version;
            instance.loader_version = None;
        })
        .await?;

    telemetry::track(&app, Event::new("pack_version_changed").instance(&updated));

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(updated)
}

#[tauri::command]
pub async fn list_pack_blocked(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<Vec<packs::BlockedFile>> {
    let paths = state.paths().await;

    Ok(
        cast_core::install::pack_files::load_blocked(&paths.instance(&instance_id).pack_blocked())
            .await,
    )
}

#[tauri::command]
pub async fn save_pack_icon(
    state: Ctx<'_>,
    provider: PackProvider,
    project_id: String,
    url: String,
) -> CommandResult<IconFile> {
    let bytes = packs::icon(provider, &url).await?;
    let paths = state.paths().await;
    let name = packs::icon_file_name(provider, &project_id, &url);

    icons::save_once(&paths.icons(), &name, &bytes).await
}

#[tauri::command]
pub async fn detect_launchers() -> CommandResult<Vec<import::DetectedLauncher>> {
    Ok(import::detect().await)
}

#[tauri::command]
pub async fn pick_launcher_dir(
    app: AppHandle,
    dialog: DialogText,
) -> CommandResult<Option<String>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title(dialog.title)
        .pick_folder(move |picked| {
            let _ = sender.send(picked);
        });

    let picked = receiver
        .await
        .ok()
        .flatten()
        .and_then(|picked| picked.into_path().ok());

    Ok(picked.map(|path| path.display().to_string()))
}

#[tauri::command]
pub async fn scan_launcher_instances(
    kind: LauncherKind,
    path: String,
) -> CommandResult<Vec<ScannedInstance>> {
    import::scan(kind, &path).await
}

#[tauri::command]
pub async fn import_launcher_instances(
    app: AppHandle,
    state: Ctx<'_>,
    request: import::ImportRequest,
) -> CommandResult<ImportReport> {
    import::run(app, state.inner().clone(), request).await
}

#[tauri::command]
pub async fn pick_modpack_file(
    app: AppHandle,
    dialog: DialogText,
) -> CommandResult<Option<String>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    app.dialog()
        .file()
        .set_title(dialog.title)
        .add_filter(dialog.filter, &packs::local::EXTENSIONS)
        .pick_file(move |picked| {
            let _ = sender.send(picked);
        });

    let picked = receiver
        .await
        .ok()
        .flatten()
        .and_then(|picked| picked.into_path().ok());

    Ok(picked.map(|path| path.display().to_string()))
}

#[tauri::command]
pub async fn inspect_modpack_file(
    state: Ctx<'_>,
    path: String,
) -> CommandResult<packs::local::LocalPack> {
    import::pack::inspect(state.inner(), &path).await
}

#[tauri::command]
pub async fn import_modpack_file(
    app: AppHandle,
    state: Ctx<'_>,
    request: import::pack::FileImportRequest,
) -> CommandResult<Instance> {
    import::pack::import(app, state.inner().clone(), request).await
}

#[tauri::command]
pub async fn cast_export_scan(
    state: Ctx<'_>,
    instance_id: String,
) -> CommandResult<cast_core::castpack::export::ExportScan> {
    crate::export::scan(state.inner(), &instance_id).await
}

#[tauri::command]
pub async fn cast_export(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
    request: cast_core::castpack::export::ExportRequest,
    dialog: DialogText,
) -> CommandResult<Option<cast_core::castpack::export::ExportResult>> {
    crate::export::export(&app, state.inner(), &instance_id, request, dialog).await
}

#[tauri::command]
pub async fn cancel_cast_export(state: Ctx<'_>) -> CommandResult<()> {
    state.exports.cancel();

    Ok(())
}

#[tauri::command]
pub async fn cancel_import(state: Ctx<'_>) -> CommandResult<()> {
    state.imports.cancel();

    Ok(())
}

#[tauri::command]
pub async fn list_minecraft_versions(state: Ctx<'_>) -> CommandResult<VersionManifest> {
    vanilla::manifest(&state.meta).await
}

#[tauri::command]
pub async fn list_fabric_versions(state: Ctx<'_>) -> CommandResult<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct LoaderVersion {
        version: String,
    }

    let loaders: Vec<LoaderVersion> = state
        .meta
        .fetch_json("https://meta.fabricmc.net/v2/versions/loader")
        .await?;

    Ok(loaders.into_iter().map(|loader| loader.version).collect())
}

#[tauri::command]
pub async fn list_forge_versions(state: Ctx<'_>) -> CommandResult<Vec<String>> {
    let xml = state
        .meta
        .fetch_bytes(cast_core::meta::forge::FORGE_METADATA)
        .await?;

    Ok(cast_core::meta::forge::parse_maven_versions(
        &String::from_utf8_lossy(&xml),
    ))
}

#[tauri::command]
pub async fn list_neoforge_versions(state: Ctx<'_>) -> CommandResult<Vec<neoforge::Release>> {
    let (metadata, legacy) = tokio::try_join!(
        state.meta.fetch_bytes(neoforge::METADATA),
        state.meta.fetch_bytes(neoforge::LEGACY_METADATA),
    )?;

    Ok(neoforge::releases(
        &String::from_utf8_lossy(&metadata),
        &String::from_utf8_lossy(&legacy),
    ))
}
