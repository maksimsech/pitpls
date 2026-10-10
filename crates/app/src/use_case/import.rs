use std::collections::BTreeSet;

use chrono::{Datelike, NaiveDate};
use pitpls_importers::{ImportError as ParseError, import, model::ImporterKind};

use super::validation::validate_year;
use crate::App;
use pitpls_db::RepositoryError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    Parse(#[from] ParseError),
    Repository(#[from] RepositoryError),
}

pub struct ImportResult {
    pub dividends: u64,
    pub cryptos: u64,
    pub interests: u64,
}

pub async fn run_import(
    app: &App,
    kind: ImporterKind,
    file: String,
) -> Result<ImportResult, Error> {
    let data = import(kind, &file).await?;

    let dividends = app.db.dividend_repo().save(&data.dividends).await?;
    add_years(app, data.dividends.iter().map(|dividend| dividend.date)).await?;
    let cryptos = app.db.crypto_repo().save(&data.cryptos).await?;
    add_years(app, data.cryptos.iter().map(|crypto| crypto.date)).await?;
    let interests = app.db.interest_repo().save(&data.interests).await?;
    add_years(app, data.interests.iter().map(|interest| interest.date)).await?;

    Ok(ImportResult {
        dividends,
        cryptos,
        interests,
    })
}

async fn add_years(
    app: &App,
    dates: impl Iterator<Item = NaiveDate>,
) -> Result<(), RepositoryError> {
    let years = dates
        .map(|date| date.year())
        .filter(|year| validate_year(*year).is_ok())
        .collect::<BTreeSet<_>>();
    let repo = app.db.year_repo();
    for year in years {
        repo.add(year).await?;
    }
    Ok(())
}
