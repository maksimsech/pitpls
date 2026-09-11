use pitpls_core::{
    rate::NbpRateProvider,
    summary::{TaxSummary, calculate},
};

use super::{error_message, validate_optional_year};
use crate::App;

pub async fn load_tax_summary(app: &App, year: Option<i32>) -> Result<TaxSummary, String> {
    validate_optional_year(year)?;
    let rates = app.db.rate_repo().load_all().await.map_err(error_message)?;
    let rate_provider = NbpRateProvider::new(rates);
    let cryptos = app
        .db
        .crypto_repo()
        .get_by_year(year)
        .await
        .map_err(error_message)?;

    let dividends = app
        .db
        .dividend_repo()
        .get_by_year(year)
        .await
        .map_err(error_message)?;

    let interests = app
        .db
        .interest_repo()
        .get_by_year(year)
        .await
        .map_err(error_message)?;
    let dividend_rounding = app
        .db
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
