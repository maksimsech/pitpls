use std::collections::{BTreeMap, BTreeSet};

use pitpls_core::{common::Currency, rate::Rate};
use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Type)]
pub struct RatesViewModel {
    currencies: Vec<Currency>,
    rows: Vec<RateDay>,
}

#[derive(Serialize, Type)]
pub struct RateDay {
    date: String,
    rates: Vec<RateValue>,
}

#[derive(Serialize, Type)]
pub struct RateValue {
    currency: Currency,
    rate: String,
}

#[tauri::command]
#[specta::specta]
pub async fn import_csv(state: State<'_, AppState>, file: String) -> Result<u64, String> {
    let rates = nbr::load_csv_rates(&file)
        .await
        .map_err(|e| e.to_string())?;

    state
        .rate_repo()
        .upload(rates.into_iter())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn import_api(state: State<'_, AppState>, year: i32) -> Result<u64, String> {
    let rates = nbr::load_api_rates(state.api_client(), year)
        .await
        .map_err(|e| e.to_string())?;

    state
        .rate_repo()
        .upload(rates.into_iter())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn reset_rates(state: State<'_, AppState>) -> Result<u64, String> {
    state.rate_repo().reset().await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn list_rates(state: State<'_, AppState>) -> Result<RatesViewModel, String> {
    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;

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
