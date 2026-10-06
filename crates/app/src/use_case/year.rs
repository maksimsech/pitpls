use super::{error_message, validate_year};
use crate::App;

/// A year for the year selector: it has records, was added by hand, or both.
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

pub async fn list_years(app: &App) -> Result<Vec<i32>, String> {
    app.db.year_repo().list().await.map_err(error_message)
}

/// Years with records merged with the custom years, newest first.
pub async fn list_year_info(app: &App) -> Result<Vec<YearInfo>, String> {
    let years = app
        .db
        .year_repo()
        .list_with_records()
        .await
        .map_err(error_message)?;
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

pub async fn add_year(app: &App, year: i32) -> Result<(), String> {
    validate_year(year)?;
    app.db.year_repo().add(year).await.map_err(error_message)
}

pub async fn delete_year(app: &App, year: i32) -> Result<u64, String> {
    validate_year(year)?;
    app.db.year_repo().delete(year).await.map_err(error_message)
}
