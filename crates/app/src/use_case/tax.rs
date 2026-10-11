use pitpls_core::{
    rate::NbpRateProvider,
    summary::{CalculateTaxSummaryError, TaxSummary, calculate},
};

use super::stock::load_listings;
use super::validation::{Error as ValidationError, validate_year};
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

pub async fn load_tax_summary(app: &App, year: i32) -> Result<TaxSummary, Error> {
    validate_year(year)?;
    let rates = app.db.rate_repo().load_all().await?;
    let rate_provider = NbpRateProvider::new(rates);
    let cryptos = app.db.crypto_repo().get_by_year(Some(year)).await?;

    let dividends = app.db.dividend_repo().get_by_year(Some(year)).await?;

    let interests = app.db.interest_repo().get_by_year(Some(year)).await?;
    let stocks = app.db.stock_repo().get_through_year(year).await?;
    let listings = load_listings(app).await?;
    let dividend_rounding = app.db.settings_repo().load_dividend_rounding().await?;

    calculate(
        year,
        &rate_provider,
        cryptos,
        dividends,
        interests,
        &stocks,
        &listings.exchanges(),
        dividend_rounding,
    )
    .map_err(Error::from)
}
