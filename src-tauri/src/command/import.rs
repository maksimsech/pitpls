use pitpls_importers::{import, model::ImporterKind};
use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Type)]
pub struct ImportResult {
    pub dividends: u64,
    pub cryptos: u64,
    pub interests: u64,
}

#[tauri::command]
#[specta::specta]
pub async fn run_import(
    state: State<'_, AppState>,
    kind: ImporterKind,
    file: String,
) -> Result<ImportResult, String> {
    let data = import(kind, &file).await.map_err(|e| e.to_string())?;

    let dividends = state
        .dividend_repo()
        .save(&data.dividends)
        .await
        .map_err(|e| e.to_string())?;
    let cryptos = state
        .crypto_repo()
        .save(&data.cryptos)
        .await
        .map_err(|e| e.to_string())?;
    let interests = state
        .interest_repo()
        .save(&data.interests)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ImportResult {
        dividends,
        cryptos,
        interests,
    })
}
