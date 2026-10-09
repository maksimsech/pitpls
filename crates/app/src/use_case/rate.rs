use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use pitpls_core::{common::Currency, rate::Rate};
use pitpls_nbr::{
    ApiImportError as NbpApiError, CsvImportError as NbpCsvError, load_api_rates, load_csv_rates,
};
use rust_decimal::Decimal;

use super::validation::{Error as ValidationError, validate_year};
use crate::App;
use pitpls_db::RepositoryError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum CsvImportError {
    Parse(#[from] NbpCsvError),
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum ApiImportError {
    Validation(#[from] ValidationError),
    Fetch(#[from] NbpApiError),
    Repository(#[from] RepositoryError),
}

pub struct RatesViewModel {
    pub currencies: Vec<Currency>,
    pub rows: Vec<RateDay>,
}

pub struct RateDay {
    pub date: NaiveDate,
    pub rates: Vec<RateValue>,
}

pub struct RateValue {
    pub currency: Currency,
    pub rate: Decimal,
}

/// The dates of the stored NBP rates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateCoverage {
    pub first: NaiveDate,
    pub last: NaiveDate,
}

pub async fn import_csv(app: &App, file: String) -> Result<u64, CsvImportError> {
    let rates = load_csv_rates(&file).await?;

    app.db
        .rate_repo()
        .upload(rates.into_iter())
        .await
        .map_err(CsvImportError::from)
}

pub async fn import_api(app: &App, year: i32) -> Result<u64, ApiImportError> {
    validate_year(year)?;
    let rates = load_api_rates(&app.api_client, year).await?;

    app.db
        .rate_repo()
        .upload(rates.into_iter())
        .await
        .map_err(ApiImportError::from)
}

pub async fn reset_rates(app: &App) -> Result<u64, RepositoryError> {
    app.db.rate_repo().reset().await
}

/// The earliest and latest rate dates, or nothing when no rates are stored.
pub async fn rate_coverage(app: &App) -> Result<Option<RateCoverage>, RepositoryError> {
    let coverage = app.db.rate_repo().coverage().await?;
    Ok(coverage.map(|(first, last)| RateCoverage { first, last }))
}

pub async fn list_rates(app: &App) -> Result<RatesViewModel, RepositoryError> {
    let rates = app.db.rate_repo().load_all().await?;

    let mut currencies = BTreeSet::new();
    let mut rates_by_day = BTreeMap::<_, Vec<RateValue>>::new();
    for rate in rates {
        if let Some(rate_value) = rate_value(&rate) {
            currencies.insert(rate_value.currency);
            rates_by_day.entry(rate.date).or_default().push(rate_value);
        }
    }

    let rows = rates_by_day
        .into_iter()
        .map(|(date, rates)| RateDay { date, rates })
        .collect();

    let mut currencies = currencies.into_iter().collect::<Vec<_>>();
    currencies.sort_by_key(|currency| currency_priority(*currency));

    Ok(RatesViewModel { currencies, rows })
}

fn rate_value(rate: &Rate) -> Option<RateValue> {
    if matches!(rate.currency, Currency::PLN) {
        return None;
    }

    Some(RateValue {
        currency: rate.currency,
        rate: rate.rate,
    })
}

fn currency_priority(currency: Currency) -> (u8, Currency) {
    match currency {
        Currency::USD => (0, currency),
        Currency::EUR => (1, currency),
        _ => (2, currency),
    }
}
