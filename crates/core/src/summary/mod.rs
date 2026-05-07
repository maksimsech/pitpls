use thiserror::Error;

use crate::{
    crypto::{CalculateSellBuyValuesError, Crypto, calculate_sell_buy_values},
    dividend::{CalculateDividendTaxError, Dividend, calculate as calculate_dividends},
    interest::{CalculateInterestTaxError, Interest, calculate as calculate_interest},
    rate::NbpRateProvider,
    settings::DividendRounding,
};

pub use model::{CryptoTaxSummary, ForeignTaxSummary, TaxSummary};

mod model;

#[derive(Debug, Error)]
pub enum CalculateTaxSummaryError {
    #[error("Failed to calculate crypto tax summary: {0}")]
    Crypto(#[from] CalculateSellBuyValuesError),
    #[error("Failed to calculate dividend tax summary: {0}")]
    Dividend(#[from] CalculateDividendTaxError),
    #[error("Failed to calculate interest tax summary: {0}")]
    Interest(#[from] CalculateInterestTaxError),
}

pub fn calculate(
    rate_provider: &NbpRateProvider,
    cryptos: Vec<Crypto>,
    dividends: Vec<Dividend>,
    interests: Vec<Interest>,
    dividend_rounding: DividendRounding,
) -> Result<TaxSummary, CalculateTaxSummaryError> {
    let crypto_tax = calculate_sell_buy_values(cryptos, rate_provider)?;
    let dividend_tax = calculate_dividends(dividends, rate_provider, dividend_rounding)?;
    let interest_tax = calculate_interest(interests, rate_provider)?;

    Ok(TaxSummary {
        crypto: CryptoTaxSummary {
            income: crypto_tax.income,
            costs: crypto_tax.costs,
        },
        foreign: ForeignTaxSummary {
            income: dividend_tax.income + interest_tax.income,
            tax_to_pay: dividend_tax.to_pay + interest_tax.to_pay,
            tax_paid: dividend_tax.paid,
        },
    })
}
