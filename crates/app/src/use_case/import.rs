use std::{collections::BTreeSet, path::Path};

use chrono::{Datelike, NaiveDate, Utc};
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

pub use pitpls_db::repository::last_import::LastImport;

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

    let count = |rows: u64| u32::try_from(rows).unwrap_or(u32::MAX);
    app.db
        .last_import_repo()
        .save(&LastImport {
            provider: kind.provider().to_owned(),
            file_name: Path::new(&file)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            dividends: count(dividends),
            interests: count(interests),
            cryptos: count(cryptos),
            imported_at: Utc::now(),
        })
        .await?;

    Ok(ImportResult {
        dividends,
        cryptos,
        interests,
    })
}

pub async fn load_last_import(app: &App) -> Result<Option<LastImport>, RepositoryError> {
    app.db.last_import_repo().load().await
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
