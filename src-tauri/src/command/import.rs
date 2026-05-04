use pitpls_importers::{import, model::ImporterKind};
use serde::Serialize;
use specta::Type;
use tauri::State;

use super::error_message;
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
    let data = import(kind, &file).await.map_err(error_message)?;

    let dividends = state
        .dividend_repo()
        .save(&data.dividends)
        .await
        .map_err(error_message)?;
    let cryptos = state
        .crypto_repo()
        .save(&data.cryptos)
        .await
        .map_err(error_message)?;
    let interests = state
        .interest_repo()
        .save(&data.interests)
        .await
        .map_err(error_message)?;

    Ok(ImportResult {
        dividends,
        cryptos,
        interests,
    })
}
