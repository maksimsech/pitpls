use anyhow::Result;
use std::collections::BTreeMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    common::{Amount, Currency},
    rate::csv::read_csv,
};

mod csv;

mod model;

pub use model::RateExport;

const MAX_LOOKUP_STEPS: u8 = 10;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RateConverterError {
    #[error("There is no rates available.")]
    NoRatesAvailable,
    #[error("It took {steps} steps to get n-1 rate for {currency} on {date}.")]
    StepLimitReached {
        steps: u8,
        currency: String,
        date: NaiveDate,
    },
}

pub struct NbpRateProvider {
    rates_by_date: BTreeMap<NaiveDate, BTreeMap<Currency, Decimal>>,
}

impl NbpRateProvider {
    pub async fn load(path: &str) -> Result<Self> {
        let csv_rates = read_csv(path).await?;

        let rates_by_date = csv_rates
            .into_iter()
            .map(|r| {
                let date = NaiveDate::parse_from_str(&r.date, "%Y%m%d")?;
                let rates = r
                    .rate_map
                    .into_iter()
                    .filter_map(|(code, rate)| {
                        let symbol = code.trim_start_matches(|c: char| c.is_ascii_digit());
                        let currency = match symbol {
                            "USD" => Currency::USD,
                            "EUR" => Currency::EUR,
                            _ => return None,
                        };

                        Some(parse_unit(code.as_str()).map(|unit| (currency, rate / unit)))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;

                Ok((date, rates))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;

        Ok(Self { rates_by_date })
    }

    pub fn new(rates: Vec<RateExport>) -> Self {
        let mut rates_by_date: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        for rate in rates {
            rates_by_date
                .entry(rate.date)
                .or_default()
                .insert(rate.currency, rate.rate);
        }

        Self { rates_by_date }
    }

    pub fn export(&self) -> impl Iterator<Item = RateExport> {
        self.rates_by_date.iter().flat_map(|(date, rates)| {
            rates.iter().map(|(currency, rate)| RateExport {
                date: *date,
                currency: *currency,
                rate: *rate,
            })
        })
    }

    pub fn convert(
        &self,
        amount: &Amount,
        at: &NaiveDate,
    ) -> std::result::Result<(Decimal, NaiveDate), RateConverterError> {
        if matches!(amount.currency, Currency::PLN) {
            return Ok((amount.value, *at));
        }

        let (rate, rate_date) = self.get(at, &amount.currency)?;

        Ok((rate * amount.value, rate_date))
    }

    fn get(
        &self,
        date: &NaiveDate,
        currency: &Currency,
    ) -> std::result::Result<(Decimal, NaiveDate), RateConverterError> {
        if self.rates_by_date.is_empty() {
            return Err(RateConverterError::NoRatesAvailable);
        }

        let mut previous = date.pred_opt().unwrap();

        for _ in 0..MAX_LOOKUP_STEPS {
            let rates_maybe = self.rates_by_date.get(&previous);

            if let Some(rates) = rates_maybe
                && let Some(rate) = rates.get(currency)
            {
                return Ok((*rate, previous));
            }

            previous = previous.pred_opt().unwrap();
        }

        Err(RateConverterError::StepLimitReached {
            steps: MAX_LOOKUP_STEPS,
            currency: currency.to_string(),
            date: *date,
        })
    }
}

fn parse_unit(code: &str) -> Result<Decimal> {
    let digits = code
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return Ok(Decimal::ONE);
    }

    Ok(digits.parse()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn usd_amount(value: i64) -> Amount {
        Amount {
            value: Decimal::from(value),
            currency: Currency::USD,
        }
    }

    #[test]
    fn convert_returns_error_when_no_rates_available() {
        let provider = NbpRateProvider::new(Vec::new());

        let error = provider
            .convert(&usd_amount(1), &date(2024, 1, 2))
            .unwrap_err();

        assert_eq!(error, RateConverterError::NoRatesAvailable);
    }

    #[test]
    fn convert_returns_error_when_lookup_exceeds_ten_steps() {
        let provider = NbpRateProvider::new(vec![RateExport {
            date: date(2024, 1, 1),
            currency: Currency::USD,
            rate: Decimal::ONE,
        }]);

        let error = provider
            .convert(&usd_amount(1), &date(2024, 1, 12))
            .unwrap_err();

        assert!(matches!(
            error,
            RateConverterError::StepLimitReached { steps: 10, .. }
        ));
    }

    #[test]
    fn convert_returns_rate_when_lookup_takes_ten_steps() {
        let provider = NbpRateProvider::new(vec![RateExport {
            date: date(2024, 1, 1),
            currency: Currency::USD,
            rate: Decimal::ONE,
        }]);

        let (value, rate_date) = provider
            .convert(&usd_amount(2), &date(2024, 1, 11))
            .unwrap();

        assert_eq!(value, Decimal::from(2));
        assert_eq!(rate_date, date(2024, 1, 1));
    }
}
