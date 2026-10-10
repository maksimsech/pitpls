use pitpls_core::{
    rate::NbpRateProvider,
    summary::{CalculateTaxSummaryError, TaxSummary, calculate},
};

use super::validation::{Error as ValidationError, validate_optional_year};
use crate::App;
use pitpls_db::RepositoryError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    Validation(#[from] ValidationError),
    Repository(#[from] RepositoryError),
    Calculation(#[from] CalculateTaxSummaryError),
}

pub async fn load_tax_summary(app: &App, year: Option<i32>) -> Result<TaxSummary, Error> {
    validate_optional_year(year)?;
    let rates = app.db.rate_repo().load_all().await?;
    let rate_provider = NbpRateProvider::new(rates);
    let cryptos = app.db.crypto_repo().get_by_year(year).await?;

    let dividends = app.db.dividend_repo().get_by_year(year).await?;

    let interests = app.db.interest_repo().get_by_year(year).await?;
    let dividend_rounding = app.db.settings_repo().load_dividend_rounding().await?;

    calculate(
        &rate_provider,
        cryptos,
        dividends,
        interests,
        dividend_rounding,
    )
    .map_err(Error::from)
}
