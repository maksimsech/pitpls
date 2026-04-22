use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Type)]
pub struct Warnings {
    rates_empty: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn get_warnings(state: State<'_, AppState>) -> Result<Warnings, String> {
    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;

    Ok(Warnings {
        rates_empty: rates.is_empty(),
    })
}
