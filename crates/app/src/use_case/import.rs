use pitpls_importers::{import, model::ImporterKind};
use serde::Serialize;
use specta::Type;

use super::error_message;
use crate::App;

#[derive(Serialize, Type)]
pub struct ImportResult {
    pub dividends: u64,
    pub cryptos: u64,
    pub interests: u64,
}

pub async fn run_import(
    app: &App,
    kind: ImporterKind,
    file: String,
) -> Result<ImportResult, String> {
    let data = import(kind, &file).await.map_err(error_message)?;

    let dividends = app
        .db
        .dividend_repo()
        .save(&data.dividends)
        .await
        .map_err(error_message)?;
    let cryptos = app
        .db
        .crypto_repo()
        .save(&data.cryptos)
        .await
        .map_err(error_message)?;
    let interests = app
        .db
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
