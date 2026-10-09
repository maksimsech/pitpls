use super::validation::{Error as ValidationError, validate_year};
use crate::App;
use pitpls_db::RepositoryError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    Validation(#[from] ValidationError),
    Repository(#[from] RepositoryError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YearInfo {
    pub year: i32,
    pub dividends: u32,
    pub interests: u32,
    pub cryptos: u32,
    /// Whether the year is stored in the custom year table.
    pub custom: bool,
}

impl YearInfo {
    pub fn records(&self) -> u32 {
        self.dividends + self.interests + self.cryptos
    }
}

pub async fn list_year_info(app: &App) -> Result<Vec<YearInfo>, RepositoryError> {
    let years = app.db.year_repo().list_with_records().await?;
    Ok(years
        .into_iter()
        .map(|year| YearInfo {
            year: year.year,
            dividends: year.dividends,
            interests: year.interests,
            cryptos: year.cryptos,
            custom: year.custom,
        })
        .collect())
}

pub async fn add_year(app: &App, year: i32) -> Result<(), Error> {
    validate_year(year)?;
    app.db.year_repo().add(year).await.map_err(Error::from)
}

pub async fn delete_year(app: &App, year: i32) -> Result<u64, Error> {
    validate_year(year)?;
    app.db.year_repo().delete(year).await.map_err(Error::from)
}
