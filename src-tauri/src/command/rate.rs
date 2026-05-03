use std::collections::BTreeMap;

use pitpls_core::common::Currency;
use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Type)]
pub struct RatesViewModel {
    rows: Vec<RateDay>,
}

#[derive(Serialize, Type)]
pub struct RateDay {
    date: String,
    usd: Option<String>,
    eur: Option<String>,
}

#[derive(Default)]
struct RateValues {
    usd: Option<String>,
    eur: Option<String>,
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
    let rates = nbr::load_api_rates(year).await.map_err(|e| e.to_string())?;

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

    let mut rates_by_day = BTreeMap::<_, RateValues>::new();
    for rate in rates {
        let day = rates_by_day.entry(rate.date).or_default();
        match rate.currency {
            Currency::USD => day.usd = Some(rate.rate.to_string()),
            Currency::EUR => day.eur = Some(rate.rate.to_string()),
            Currency::PLN => {}
        }
    }

    let rows = rates_by_day
        .into_iter()
        .map(|(date, rates)| RateDay {
            date: date.to_string(),
            usd: rates.usd,
            eur: rates.eur,
        })
        .collect();

    Ok(RatesViewModel { rows })
}
