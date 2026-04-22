use pitpls_core::{common::Currency, rate::NbpRateProvider};
use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Type)]
pub struct Rate {
    date: String,
    currency: Currency,
    rate: String,
}

#[tauri::command]
#[specta::specta]
pub async fn upload_rates(state: State<'_, AppState>, file: String) -> Result<u64, String> {
    let rate_provider = NbpRateProvider::load(&file)
        .await
        .map_err(|e| e.to_string())?;

    state
        .rate_repo()
        .upload(rate_provider.export())
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
pub async fn list_rates(state: State<'_, AppState>) -> Result<Vec<Rate>, String> {
    let mut rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;

    rates.sort_unstable_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.currency.cmp(&b.currency))
    });

    Ok(rates
        .into_iter()
        .map(|r| Rate {
            date: r.date.to_string(),
            currency: r.currency,
            rate: r.rate.to_string(),
        })
        .collect())
}
