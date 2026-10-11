use thiserror::Error;

use crate::{
    crypto::{CalculateSellBuyValuesError, Crypto, calculate_sell_buy_values},
    dividend::{CalculateDividendTaxError, Dividend, calculate as calculate_dividends},
    interest::{CalculateInterestTaxError, Interest, calculate as calculate_interest},
    rate::NbpRateProvider,
    settings::DividendRounding,
    stock::{CalculateStocksError, InstrumentExchange, Stock, calculate_stocks},
};

pub use model::{CryptoTaxSummary, ForeignTaxSummary, StockTaxSummary, TaxSummary};

mod model;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum CalculateTaxSummaryError {
    Crypto(#[from] CalculateSellBuyValuesError),
    Dividend(#[from] CalculateDividendTaxError),
    Interest(#[from] CalculateInterestTaxError),
    Stock(#[from] CalculateStocksError),
}

// Stocks go through the year, since earlier lots and late-December sales count in it.
#[allow(clippy::too_many_arguments)]
pub fn calculate(
    year: i32,
    rate_provider: &NbpRateProvider,
    cryptos: Vec<Crypto>,
    dividends: Vec<Dividend>,
    interests: Vec<Interest>,
    stocks: &[Stock],
    exchanges: &[InstrumentExchange],
    dividend_rounding: DividendRounding,
) -> Result<TaxSummary, CalculateTaxSummaryError> {
    let crypto_tax = calculate_sell_buy_values(cryptos, rate_provider)?;
    let dividend_tax = calculate_dividends(dividends, rate_provider, dividend_rounding)?;
    let interest_tax = calculate_interest(interests, rate_provider)?;
    let stock_tax = calculate_stocks(stocks, exchanges, year, rate_provider)?;

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
        stocks: StockTaxSummary {
            income: stock_tax.income,
            costs: stock_tax.costs,
            tax_paid_abroad: stock_tax.tax_paid_abroad,
            countries: stock_tax.countries,
        },
    })
}
