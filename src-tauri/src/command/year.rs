use tauri::State;

use super::error_message;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn list_years(state: State<'_, AppState>) -> Result<Vec<i32>, String> {
    state.year_repo().list().await.map_err(error_message)
}

#[tauri::command]
#[specta::specta]
pub async fn add_year(state: State<'_, AppState>, year: i32) -> Result<(), String> {
    if !(1900..=2100).contains(&year) {
        return Err(format!("Year {year} out of range (1900-2100)"));
    }
    state.year_repo().add(year).await.map_err(error_message)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_year(state: State<'_, AppState>, year: i32) -> Result<u64, String> {
    state.year_repo().delete(year).await.map_err(error_message)
}
