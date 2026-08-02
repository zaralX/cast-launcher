pub mod process;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use args::LaunchInputs;
use cast_core::launch::game::RunningGame;
use process::SpawnOptions;
use tauri::AppHandle;

use cast_core::error::{CommandError, CommandResult};
use cast_core::fs_util::{ensure_dir, remove_dir_if_exists};
use cast_core::instance::Instance;
use cast_core::java::{self, ResolveOptions};
use cast_core::launch::args;
use cast_core::meta::Resolver;

use crate::events::{EmitExt, LauncherEvent};
use crate::state::AppState;
use crate::telemetry::{self, Event};

pub async fn launch(
    app: AppHandle,
    state: Arc<AppState>,
    instance_id: &str,
) -> CommandResult<RunningGame> {
    match run(app.clone(), state, instance_id).await {
        Ok(game) => Ok(game),
        Err(error) => {
            telemetry::track(&app, Event::new("launch_failed").error(&error));
            Err(error)
        }
    }
}

async fn run(
    app: AppHandle,
    state: Arc<AppState>,
    instance_id: &str,
) -> CommandResult<RunningGame> {
    let instance = state.instances.get(instance_id).await?;

    if !instance.installed {
        return Err(CommandError::launch(format!(
            "Сборка «{}» ещё не установлена",
            instance.name
        )));
    }

    if state.processes.is_running(&instance.id).await {
        return Err(CommandError::launch(format!(
            "Сборка «{}» уже запущена",
            instance.name
        )));
    }

    let account = state.accounts.active_for_launch().await?;
    let paths = state.paths().await;
    let config = instance.effective_config(&state.config().await);

    let resolver = Resolver::new(&paths, &state.meta);
    let base = resolver.base_package(&instance).await?;
    let requirement = resolver.java_requirement(&base);

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

    let ctx = java.runtime_context();
    let profile = resolver.profile(&instance, &base, &ctx).await?;

    if !profile.main_jar.path.is_file() {
        state
            .instances
            .update(&paths, &instance.id, |instance| instance.installed = false)
            .await
            .ok();

        return Err(CommandError::launch(format!(
            "Файлы сборки «{}» неполные, установите её заново",
            instance.name
        ))
        .with_details(profile.main_jar.path.display().to_string()));
    }

    let instance_paths = paths.instance(&instance.id);
    ensure_dir(&instance_paths.minecraft()).await?;

    let natives_dir = prepare_natives(&paths, &instance, &profile).await?;

    let command = args::build(&LaunchInputs {
        paths: &paths,
        instance: &instance_paths,
        profile: &profile,
        config: &config,
        account: &account,
        java_path: &java.path,
        ctx: &ctx,
        natives_dir: &natives_dir,
    });

    let game = state
        .processes
        .spawn(
            app.clone(),
            instance.id.clone(),
            instance.name.clone(),
            command,
            SpawnOptions {
                log_path: Some(
                    paths
                        .instance_logs(&instance.id)
                        .join(format!("{}.log", timestamp())),
                ),
                cleanup_dir: Some(natives_dir),
            },
        )
        .await?;

    telemetry::track(
        &app,
        Event::new("game_launched")
            .instance(&instance)
            .num("java_major", java.major)
            .text("java_source", java.source)
            .text("java_mode", config.java.java_mode.key())
            .num("min_ram", config.java.min_ram)
            .num("max_ram", config.java.max_ram)
            .flag("first_launch", instance.playtime.total_seconds == 0)
            .flag("overrides", instance.settings.overrides_anything()),
    );

    if let Err(error) = state
        .instances
        .record_launch(&paths, &instance.id, game.started_at)
        .await
    {
        eprintln!("Не удалось отметить запуск сборки: {}", error.message);
    }

    LauncherEvent::Instances {
        instances: state.instances.all().await,
    }
    .emit(&app);

    Ok(game)
}

async fn prepare_natives(
    paths: &cast_core::paths::LauncherPaths,
    instance: &Instance,
    profile: &cast_core::mojang::profile::ResolvedProfile,
) -> CommandResult<std::path::PathBuf> {
    let dir = paths.instance(&instance.id).natives();

    remove_dir_if_exists(&dir).await;
    ensure_dir(&dir).await?;

    for jar in args::native_jars(paths, profile) {
        if !jar.is_file() {
            return Err(CommandError::fs(format!(
                "Нативная библиотека не найдена: {}",
                jar.display()
            ))
            .with_details("Переустановите сборку"));
        }

        cast_core::archive::extract_natives(jar, dir.clone()).await?;
    }

    Ok(dir)
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
        .to_string()
}
