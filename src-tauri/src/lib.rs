mod castpack;
mod commands;
mod events;
mod import;
mod install;
mod launch;
mod play;
mod state;
mod telemetry;
mod window;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime = tauri::async_runtime::handle();
    let _runtime_guard = runtime.inner().enter();

    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        window::focus_or_create(app);
    }));

    builder
        .plugin(telemetry::plugin())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let state = state::AppState::initialize(&handle).await?;

                telemetry::set_enabled(state.config().await.launcher.telemetry);
                handle.manage(std::sync::Arc::clone(&state));
                telemetry::app_started(&handle, &state).await;

                Ok::<_, cast_core::error::CommandError>(())
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::get_config,
            commands::update_config,
            commands::get_paths,
            commands::open_path,
            commands::list_instances,
            commands::reload_instances,
            commands::create_instance,
            commands::update_instance,
            commands::delete_instance,
            commands::open_instance_dir,
            commands::list_instance_logs,
            commands::read_instance_log,
            commands::delete_instance_log,
            commands::list_icons,
            commands::read_icon,
            commands::import_icon,
            commands::delete_icon,
            commands::list_item_icons,
            commands::item_icons,
            commands::save_item_icon,
            commands::install_instance,
            commands::cancel_install,
            commands::awaited_files,
            commands::downloads_dir,
            commands::scan_for_files,
            commands::rescan_files,
            commands::pick_folder,
            commands::resume_install,
            commands::list_installs,
            commands::launch_instance,
            commands::play_instance,
            commands::list_running,
            commands::stop_instance,
            commands::list_java,
            commands::probe_java,
            commands::list_accounts,
            commands::select_account,
            commands::remove_account,
            commands::add_offline_account,
            commands::login_microsoft,
            commands::refresh_account,
            commands::castpack_catalog,
            commands::castpack_install,
            commands::castpack_check_update,
            commands::castpack_set_autoupdate,
            commands::castpack_validate,
            commands::castpack_save_manifest,
            commands::castpack_probe_file,
            commands::castpack_probe_mod,
            commands::open_url,
            commands::pack_providers,
            commands::search_packs,
            commands::list_pack_versions,
            commands::pack_filters,
            commands::set_instance_pack_version,
            commands::list_pack_blocked,
            commands::save_pack_icon,
            commands::detect_launchers,
            commands::pick_launcher_dir,
            commands::scan_launcher_instances,
            commands::import_launcher_instances,
            commands::cancel_import,
            commands::pick_modpack_file,
            commands::inspect_modpack_file,
            commands::import_modpack_file,
            commands::list_minecraft_versions,
            commands::list_fabric_versions,
            commands::list_forge_versions,
            commands::list_neoforge_versions,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { code, api, .. } => {
                if code.is_none() && window::supervising(app) {
                    api.prevent_exit();
                }
            }

            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { has_visible_windows, .. } => {
                if !has_visible_windows {
                    window::focus_or_create(app);
                }
            }

            tauri::RunEvent::Exit => telemetry::app_exited(app),

            _ => {}
        });
}
