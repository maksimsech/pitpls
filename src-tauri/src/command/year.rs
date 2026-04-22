use std::collections::HashSet;

use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Type)]
pub struct YearOption {
    pub year: i32,
    pub explicit: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_years(state: State<'_, AppState>) -> Result<Vec<YearOption>, String> {
    let explicit: HashSet<i32> = state
        .year_repo()
        .list()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let all = state
        .year_repo()
        .list_dropdown()
        .await
        .map_err(|e| e.to_string())?;
    Ok(all
        .into_iter()
        .map(|year| YearOption {
            year,
            explicit: explicit.contains(&year),
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn add_year(state: State<'_, AppState>, year: i32) -> Result<(), String> {
    if !(1900..=2100).contains(&year) {
        return Err(format!("Year {year} out of range (1900-2100)"));
    }
    state
        .year_repo()
        .add(year)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_year(state: State<'_, AppState>, year: i32) -> Result<u64, String> {
    state
        .year_repo()
        .delete(year)
        .await
        .map_err(|e| e.to_string())
}
