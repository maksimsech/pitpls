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
            countries: stock_tax.countries,
            warnings: stock_tax.warnings,
        },
    })
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate, TimeZone, Utc};
    use rust_decimal::{Decimal, dec};

    use super::*;
    use crate::{
        common::{Amount, Country, Currency},
        crypto::Action,
        rate::Rate,
    };

    fn usd(value: Decimal) -> Amount {
        Amount {
            value,
            currency: Currency::USD,
        }
    }

    #[test]
    fn foreign_income_adds_interest_and_credit_is_dividends_only() {
        let rates = NbpRateProvider::new(
            NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .iter_days()
                .take_while(|day| day.year() == 2025)
                .map(|date| Rate {
                    date,
                    currency: Currency::USD,
                    rate: dec!(4),
                })
                .collect(),
        );
        let time = Utc.with_ymd_and_hms(2025, 5, 15, 12, 0, 0).unwrap();
        let dividends = vec![Dividend {
            id: "dividend".to_owned(),
            time,
            ticker: "KO".to_owned(),
            value: usd(dec!(100)),
            tax_paid: usd(dec!(15)),
            country: Country::USA,
            provider: "Revolut".to_owned(),
        }];
        let interests = vec![Interest {
            id: "interest".to_owned(),
            time,
            value: usd(dec!(10)),
            provider: "Trading 212".to_owned(),
        }];
        let cryptos = vec![Crypto {
            id: "crypto".to_owned(),
            value: usd(dec!(50)),
            fee: usd(dec!(1)),
            action: Action::FiatSell,
            time,
            provider: "Coinbase".to_owned(),
        }];

        let summary = calculate(
            2025,
            &rates,
            cryptos,
            dividends,
            interests,
            &[],
            &[],
            DividendRounding::SumToGroszy,
        )
        .unwrap();

        assert_eq!(
            (
                summary.foreign.income,
                summary.foreign.tax_to_pay,
                summary.foreign.tax_paid
            ),
            (dec!(440), dec!(83.6), dec!(60))
        );
        assert_eq!(
            (summary.crypto.income, summary.crypto.costs),
            (dec!(200), dec!(4))
        );
        assert_eq!(
            (summary.stocks.income, summary.stocks.costs),
            (Decimal::ZERO, Decimal::ZERO)
        );
    }
}
