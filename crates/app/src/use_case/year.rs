use super::stock;
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
    pub stocks: u32,
    /// Whether the year is stored in the custom year table.
    pub custom: bool,
}

impl YearInfo {
    pub fn records(&self) -> u32 {
        self.dividends + self.interests + self.cryptos + self.stocks
    }
}

pub async fn list_year_info(app: &App) -> Result<Vec<YearInfo>, RepositoryError> {
    let years = app.db.year_repo().list_with_records().await?;
    let mut years = years
        .into_iter()
        .map(|year| YearInfo {
            year: year.year,
            dividends: year.dividends,
            interests: year.interests,
            cryptos: year.cryptos,
            stocks: year.stocks,
            custom: year.custom,
        })
        .collect::<Vec<_>>();

    // A sale counts in the year it settles, so one traded on 30.12 can be all the next year has.
    if let Some(last) = years
        .iter()
        .filter(|info| info.stocks > 0)
        .map(|info| info.year)
        .max()
    {
        for year in stock::sale_years(app, last).await? {
            if !years.iter().any(|info| info.year == year) {
                years.push(YearInfo {
                    year,
                    dividends: 0,
                    interests: 0,
                    cryptos: 0,
                    stocks: 0,
                    custom: false,
                });
            }
        }
        years.sort_by_key(|info| std::cmp::Reverse(info.year));
    }
    Ok(years)
}

pub async fn add_year(app: &App, year: i32) -> Result<(), Error> {
    validate_year(year)?;
    app.db.year_repo().add(year).await.map_err(Error::from)
}

pub async fn delete_year(app: &App, year: i32) -> Result<u64, Error> {
    validate_year(year)?;
    // A year may exist only because a sale from the previous year settles in it.
    // Use the same settlement rules as the year list, without requiring NBP rates.
    let records = app.db.stock_repo().get_through_year(year).await?;
    let exchanges = stock::load_listings(app).await?.exchanges();
    let settled = records
        .iter()
        .filter(|record| pitpls_core::stock::sale_year(record, &exchanges) == Some(year))
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();
    app.db
        .year_repo()
        .delete(Some(year), &settled)
        .await
        .map_err(Error::from)
}

pub async fn delete_all_years(app: &App) -> Result<u64, Error> {
    app.db
        .year_repo()
        .delete(None, &[])
        .await
        .map_err(Error::from)
}
