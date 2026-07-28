//! Обвязка Tauri поверх `cast-core`: команды, события во фронт, состояние
//! приложения и надзор за процессом игры. Логика лежит в core и тестируется там.

mod commands;
mod events;
mod install;
mod launch;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
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
            commands::delete_instance,
            commands::install_instance,
            commands::cancel_install,
            commands::list_installs,
            commands::launch_instance,
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
            commands::load_my_packs,
            commands::list_minecraft_versions,
            commands::list_fabric_versions,
            commands::list_forge_versions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
