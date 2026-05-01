use std::collections::BTreeMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::common::{Amount, Currency};

mod model;

pub use model::Rate;

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
    pub fn new(rates: Vec<Rate>) -> Self {
        let mut rates_by_date: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        for rate in rates {
            rates_by_date
                .entry(rate.date)
                .or_default()
                .insert(rate.currency, rate.rate);
        }

        Self { rates_by_date }
    }

    pub fn export(&self) -> impl Iterator<Item = Rate> {
        self.rates_by_date.iter().flat_map(|(date, rates)| {
            rates.iter().map(|(currency, rate)| Rate {
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
        let provider = NbpRateProvider::new(vec![Rate {
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
        let provider = NbpRateProvider::new(vec![Rate {
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
