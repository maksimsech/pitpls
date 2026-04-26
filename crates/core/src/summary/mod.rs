use anyhow::Result;

use crate::{
    crypto::{Crypto, calculate_sell_buy_values},
    dividend::{Dividend, calculate as calculate_dividends},
    interest::{Interest, calculate as calculate_interest},
    rate::NbpRateProvider,
    settings::DividendRounding,
};

pub use model::{CryptoTaxSummary, ForeignTaxSummary, TaxSummary};

mod model;

pub fn calculate(
    rate_provider: &NbpRateProvider,
    cryptos: Vec<Crypto>,
    dividends: Vec<Dividend>,
    interests: Vec<Interest>,
    dividend_rounding: DividendRounding,
) -> Result<TaxSummary> {
    let crypto_tax = calculate_sell_buy_values(cryptos, rate_provider)?;
    let dividend_tax = calculate_dividends(dividends, rate_provider, dividend_rounding)?;
    let interest_tax = calculate_interest(interests, rate_provider)?;

    Ok(TaxSummary {
        crypto: CryptoTaxSummary {
            income: crypto_tax.income,
            costs: crypto_tax.costs,
        },
        foreign: ForeignTaxSummary {
            income: (dividend_tax.income + interest_tax.income),
            tax_to_pay: (dividend_tax.to_pay + interest_tax.to_pay),
            tax_paid: dividend_tax.paid,
        },
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rust_decimal::{Decimal, dec};

    use super::calculate;
    use crate::{
        common::{Amount, Country, Currency},
        crypto::{Action, Crypto},
        dividend::Dividend,
        interest::Interest,
        rate::{NbpRateProvider, RateExport},
        settings::DividendRounding,
    };

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn pln(value: Decimal) -> Amount {
        Amount {
            value,
            currency: Currency::PLN,
        }
    }

    fn usd(value: Decimal) -> Amount {
        Amount {
            value,
            currency: Currency::USD,
        }
    }

    #[test]
    fn calculate_summary_combines_crypto_and_foreign_taxes() {
        let rate_provider = NbpRateProvider::new(Vec::new());
        let summary = calculate(
            &rate_provider,
            vec![
                Crypto {
                    id: "sell".into(),
                    value: pln(dec!(40)),
                    fee: pln(dec!(2)),
                    action: Action::FiatSell,
                    date: date(2024, 1, 2),
                    provider: "broker".into(),
                },
                Crypto {
                    id: "buy".into(),
                    value: pln(dec!(25)),
                    fee: pln(dec!(1)),
                    action: Action::FiatBuy,
                    date: date(2024, 1, 1),
                    provider: "broker".into(),
                },
            ],
            vec![Dividend {
                id: "dividend".into(),
                date: date(2024, 1, 3),
                ticker: "ABC".into(),
                value: pln(dec!(100)),
                tax_paid: pln(dec!(10)),
                country: Country::USA,
                provider: "broker".into(),
            }],
            vec![Interest {
                id: "interest".into(),
                date: date(2024, 1, 4),
                value: pln(dec!(50)),
                provider: "bank".into(),
            }],
            DividendRounding::SumToGroszy,
        )
        .unwrap();

        assert_eq!(summary.crypto.income, dec!(40));
        assert_eq!(summary.crypto.costs, dec!(28));
        assert_eq!(summary.foreign.income, dec!(150));
        assert_eq!(summary.foreign.tax_to_pay, dec!(28.50));
        assert_eq!(summary.foreign.tax_paid, dec!(10));
    }

    #[test]
    fn calculate_summary_reports_full_foreign_tax_liability_separately_from_paid_tax() {
        let rate_provider = NbpRateProvider::new(Vec::new());
        let summary = calculate(
            &rate_provider,
            Vec::new(),
            vec![Dividend {
                id: "dividend".into(),
                date: date(2024, 1, 3),
                ticker: "ABC".into(),
                value: pln(dec!(100)),
                tax_paid: pln(dec!(10)),
                country: Country::USA,
                provider: "broker".into(),
            }],
            Vec::new(),
            DividendRounding::SumToGroszy,
        )
        .unwrap();

        assert_eq!(summary.foreign.income, dec!(100));
        assert_eq!(summary.foreign.tax_to_pay, dec!(19));
        assert_eq!(summary.foreign.tax_paid, dec!(10));
    }

    #[test]
    fn calculate_summary_uses_capped_dividend_tax_paid_in_foreign_summary() {
        let rate_provider = NbpRateProvider::new(Vec::new());
        let summary = calculate(
            &rate_provider,
            Vec::new(),
            vec![Dividend {
                id: "dividend".into(),
                date: date(2024, 1, 3),
                ticker: "ABC".into(),
                value: pln(dec!(100)),
                tax_paid: pln(dec!(30)),
                country: Country::USA,
                provider: "broker".into(),
            }],
            Vec::new(),
            DividendRounding::SumToGroszy,
        )
        .unwrap();

        assert_eq!(summary.foreign.income, dec!(100));
        assert_eq!(summary.foreign.tax_to_pay, dec!(19));
        assert_eq!(summary.foreign.tax_paid, dec!(15));
    }

    #[test]
    fn calculate_summary_uses_provided_rate_provider_for_non_pln_amounts() {
        let rate_provider = NbpRateProvider::new(vec![RateExport {
            date: date(2024, 1, 2),
            currency: Currency::USD,
            rate: dec!(4),
        }]);

        let summary = calculate(
            &rate_provider,
            Vec::new(),
            vec![Dividend {
                id: "dividend".into(),
                date: date(2024, 1, 3),
                ticker: "ABC".into(),
                value: usd(dec!(10)),
                tax_paid: usd(dec!(0)),
                country: Country::USA,
                provider: "broker".into(),
            }],
            Vec::new(),
            DividendRounding::SumToGroszy,
        )
        .unwrap();

        assert_eq!(summary.foreign.income, dec!(40));
        assert_eq!(summary.foreign.tax_to_pay, dec!(7.60));
        assert_eq!(summary.foreign.tax_paid, dec!(0));
    }
}
