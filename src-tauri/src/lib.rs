mod castpack;
mod commands;
mod events;
mod import;
mod install;
mod launch;
mod play;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let state = state::AppState::initialize(&handle).await?;
                handle.manage(state);
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
            commands::scan_prism_instances,
            commands::import_prism_instances,
            commands::cancel_import,
            commands::list_minecraft_versions,
            commands::list_fabric_versions,
            commands::list_forge_versions,
            commands::list_neoforge_versions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
