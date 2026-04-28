use pitpls_core::settings::Settings;
use tauri::State;

use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn load_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state
        .settings_repo()
        .load()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    state
        .settings_repo()
        .save(settings)
        .await
        .map_err(|e| e.to_string())
}
