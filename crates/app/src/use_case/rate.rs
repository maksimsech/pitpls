use std::collections::{BTreeMap, BTreeSet};

use pitpls_core::{common::Currency, rate::Rate};
use pitpls_nbr::{load_api_rates, load_csv_rates};
use serde::Serialize;
use specta::Type;

use super::{error_message, validate_year};
use crate::App;

#[derive(Serialize, Type)]
pub struct RatesViewModel {
    pub currencies: Vec<Currency>,
    pub rows: Vec<RateDay>,
}

#[derive(Serialize, Type)]
pub struct RateDay {
    pub date: String,
    pub rates: Vec<RateValue>,
}

#[derive(Serialize, Type)]
pub struct RateValue {
    pub currency: Currency,
    pub rate: String,
}

pub async fn import_csv(app: &App, file: String) -> Result<u64, String> {
    let rates = load_csv_rates(&file).await.map_err(error_message)?;

    app.db
        .rate_repo()
        .upload(rates.into_iter())
        .await
        .map_err(error_message)
}

pub async fn import_api(app: &App, year: i32) -> Result<u64, String> {
    validate_year(year)?;
    let rates = load_api_rates(&app.api_client, year)
        .await
        .map_err(error_message)?;

    app.db
        .rate_repo()
        .upload(rates.into_iter())
        .await
        .map_err(error_message)
}

pub async fn reset_rates(app: &App) -> Result<u64, String> {
    app.db.rate_repo().reset().await.map_err(error_message)
}

pub async fn list_rates(app: &App) -> Result<RatesViewModel, String> {
    let rates = app.db.rate_repo().load_all().await.map_err(error_message)?;

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
        .map(|(date, rates)| RateDay {
            date: date.to_string(),
            rates,
        })
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
        rate: rate.rate.to_string(),
    })
}

fn currency_priority(currency: Currency) -> (u8, Currency) {
    match currency {
        Currency::USD => (0, currency),
        Currency::EUR => (1, currency),
        _ => (2, currency),
    }
}
