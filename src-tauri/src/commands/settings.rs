use tauri::{AppHandle, State};
use crate::{
    state::app_state::AppState,
    config::{save_config},
    config::schema::AppConfig,
};

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
