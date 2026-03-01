use crate::config::AppConfig;
use crate::state::{AppState, DbAccess};

#[tauri::command]
pub async fn get_config(
    state: tauri::State<'_, AppState>,
) -> Result<AppConfig, String> {
    state.db(|conn| {
        crate::config::get_config_from_db(conn)
    })
}

#[tauri::command]
pub async fn update_config(
    state: tauri::State<'_, AppState>,
    config: AppConfig,
) -> Result<(), String> {
    state.db(|conn| {
        crate::config::update_config_in_db(conn, &config)
    })
}
