use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use cast_core::account::{Account, AccountConfig};
use cast_core::config::AppConfig;
use cast_core::error::{CommandError, CommandResult};
use cast_core::instance::{Instance, InstanceSettings};
use cast_core::java::detect::JavaRuntime;
use cast_core::logs::{self, LogFile};
use cast_core::meta::vanilla;
use cast_core::mojang::version::VersionManifest;
use cast_core::paths::PathsSnapshot;

use crate::events::{EmitExt, LauncherEvent};
use crate::install::{self, InstallSnapshot};
use cast_core::launch::game::RunningGame;
use crate::state::AppState;

type Ctx<'a> = State<'a, Arc<AppState>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub config: AppConfig,
    pub paths: PathsSnapshot,
    pub accounts: AccountConfig,
    pub instances: Vec<Instance>,
    pub installs: Vec<InstallSnapshot>,
    pub running: Vec<RunningGame>,
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
    })
}

#[tauri::command]
pub async fn get_config(state: Ctx<'_>) -> CommandResult<AppConfig> {
    Ok(state.config().await)
}

#[tauri::command]
pub async fn update_config(state: Ctx<'_>, config: AppConfig) -> CommandResult<AppConfig> {
    state.update_config(config).await
}

#[tauri::command]
pub async fn get_paths(state: Ctx<'_>) -> CommandResult<PathsSnapshot> {
    Ok(PathsSnapshot::from(&state.paths().await))
}

#[tauri::command]
pub async fn open_path(app: AppHandle, path: String) -> CommandResult<()> {
    open(&app, Path::new(&path))
}

fn open(app: &AppHandle, path: &Path) -> CommandResult<()> {
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| {
        CommandError::fs(format!("Не удалось открыть {}", path.display())).with_details(e.to_string())
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

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(created)
}

#[tauri::command]
pub async fn delete_instance(app: AppHandle, state: Ctx<'_>, instance_id: String) -> CommandResult<()> {
    if state.processes.is_running(&instance_id).await {
        return Err(CommandError::launch("Сначала закройте запущенную игру"));
    }

    state.installs.cancel(&instance_id).await;

    let paths = state.paths().await;
    state.instances.remove(&paths, &instance_id).await?;

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<InstanceSettings>,
}

#[tauri::command]
pub async fn update_instance(
    app: AppHandle,
    state: Ctx<'_>,
    instance_id: String,
    update: InstanceUpdate,
) -> CommandResult<Instance> {
    let name = match update.name {
        Some(name) => {
            let name = name.trim().to_string();

            if name.is_empty() {
                return Err(CommandError::fs("Название сборки не может быть пустым"));
            }

            Some(name)
        }
        None => None,
    };

    let description = update.description.map(|text| text.trim().to_string());
    let settings = update.settings;

    let paths = state.paths().await;

    let updated = state
        .instances
        .update(&paths, &instance_id, move |instance| {
            if let Some(name) = name {
                instance.name = name;
            }
            if let Some(description) = description {
                instance.description = description;
            }
            if let Some(settings) = settings {
                instance.settings = settings;
            }
        })
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
        InstanceDir::Logs => paths.instance_logs(&instance_id),
    };

    cast_core::fs_util::ensure_dir(&dir).await?;

    open(&app, &dir)
}

#[tauri::command]
pub async fn list_instance_logs(state: Ctx<'_>, instance_id: String) -> CommandResult<Vec<LogFile>> {
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
    state.downloads.cancel_prefix(&install::job_prefix(&instance_id));

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
pub async fn remove_account(state: Ctx<'_>, uuid: String) -> CommandResult<AccountConfig> {
    state.accounts.remove(&uuid).await
}

#[tauri::command]
pub async fn add_offline_account(state: Ctx<'_>, name: String) -> CommandResult<AccountConfig> {
    state.accounts.add_offline(&name).await
}

#[tauri::command]
pub async fn login_microsoft(app: AppHandle, state: Ctx<'_>) -> CommandResult<Account> {
    let opener = app.clone();

    let account = cast_core::account::oauth::login(move |url| {
        opener.opener().open_url(url, None::<&str>).map_err(|e| {
            CommandError::auth("Не удалось открыть браузер для входа").with_details(e.to_string())
        })
    })
    .await?;

    state.accounts.upsert(account.clone()).await?;

    Ok(account)
}

#[tauri::command]
pub async fn refresh_account(state: Ctx<'_>, uuid: String) -> CommandResult<Account> {
    state.accounts.refresh(&uuid).await
}

#[tauri::command]
pub async fn load_my_packs(state: Ctx<'_>) -> CommandResult<serde_json::Value> {
    state
        .meta
        .fetch_json("https://s3.zaralx.ru/launcher/my_packs.json")
        .await
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
        .fetch_bytes("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")
        .await?;

    Ok(cast_core::meta::forge::parse_maven_versions(&String::from_utf8_lossy(&xml)))
}
