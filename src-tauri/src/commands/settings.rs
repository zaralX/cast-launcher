use crate::{config::save_config, config::schema::AppConfig, state::app_state::AppState};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn update_config(
    app: AppHandle,
    state: State<AppState>,
    new_config: AppConfig,
) -> Result<(), String> {
    {
        let mut config = state.config.lock().unwrap();
        *config = new_config.clone();
    }

    save_config(&app, &new_config)?;
    Ok(())
}
