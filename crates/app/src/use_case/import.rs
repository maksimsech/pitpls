use std::collections::BTreeSet;

use chrono::{Datelike, NaiveDate};
use pitpls_importers::{import, model::ImporterKind};
use serde::Serialize;
use specta::Type;

use super::{error_message, validate_year};
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
    add_years(app, data.dividends.iter().map(|dividend| dividend.date)).await?;
    let cryptos = app
        .db
        .crypto_repo()
        .save(&data.cryptos)
        .await
        .map_err(error_message)?;
    add_years(app, data.cryptos.iter().map(|crypto| crypto.date)).await?;
    let interests = app
        .db
        .interest_repo()
        .save(&data.interests)
        .await
        .map_err(error_message)?;
    add_years(app, data.interests.iter().map(|interest| interest.date)).await?;

    Ok(ImportResult {
        dividends,
        cryptos,
        interests,
    })
}

async fn add_years(app: &App, dates: impl Iterator<Item = NaiveDate>) -> Result<(), String> {
    let years = dates
        .map(|date| date.year())
        .filter(|year| validate_year(*year).is_ok())
        .collect::<BTreeSet<_>>();
    let repo = app.db.year_repo();
    for year in years {
        repo.add(year).await.map_err(error_message)?;
    }
    Ok(())
}
