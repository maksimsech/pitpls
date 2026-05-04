use pitpls_core::{
    rate::NbpRateProvider,
    summary::{TaxSummary, calculate},
};
use tauri::State;

use super::error_message;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn load_tax_summary(
    state: State<'_, AppState>,
    year: Option<i32>,
) -> Result<TaxSummary, String> {
    let rates = state.rate_repo().load_all().await.map_err(error_message)?;
    let rate_provider = NbpRateProvider::new(rates);
    let cryptos = state
        .crypto_repo()
        .get_by_year(year)
        .await
        .map_err(error_message)?;

    let dividends = state
        .dividend_repo()
        .get_by_year(year)
        .await
        .map_err(error_message)?;

    let interests = state
        .interest_repo()
        .get_by_year(year)
        .await
        .map_err(error_message)?;
    let dividend_rounding = state
        .settings_repo()
        .load_dividend_rounding()
        .await
        .map_err(error_message)?;

    calculate(
        &rate_provider,
        cryptos,
        dividends,
        interests,
        dividend_rounding,
    )
    .map_err(error_message)
}
