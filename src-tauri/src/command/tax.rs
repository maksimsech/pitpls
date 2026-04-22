use pitpls_core::{
    rate::NbpRateProvider,
    summary::{TaxSummary, calculate},
};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn load_tax_summary(state: State<'_, AppState>) -> Result<TaxSummary, String> {
    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;
    let rate_provider = NbpRateProvider::new(rates);
    let cryptos = state
        .crypto_repo()
        .get_all()
        .await
        .map_err(|e| e.to_string())?;

    let dividends = state
        .dividend_repo()
        .get_all()
        .await
        .map_err(|e| e.to_string())?;

    let interests = state
        .interest_repo()
        .get_all()
        .await
        .map_err(|e| e.to_string())?;

    calculate(&rate_provider, cryptos, dividends, interests).map_err(|e| e.to_string())
}
