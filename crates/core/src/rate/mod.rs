use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use thiserror::Error;

use crate::common::{Amount, Currency};

mod model;

pub use model::{Conversion, Rate};

const MAX_LOOKUP_STEPS: u8 = 10;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("{self:?}")]
pub enum RateConverterError {
    NoRatesAvailable,
    StepLimitReached {
        steps: u8,
        currency: Currency,
        date: NaiveDate,
        missing_year: i32,
    },
}

impl RateConverterError {
    pub fn missing_year(&self) -> Option<i32> {
        match self {
            Self::StepLimitReached { missing_year, .. } => Some(*missing_year),
            Self::NoRatesAvailable => None,
        }
    }
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

    pub fn lookup_window(at: NaiveDate) -> (NaiveDate, NaiveDate) {
        (
            at - chrono::Days::new(MAX_LOOKUP_STEPS.into()),
            at - chrono::Days::new(1),
        )
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
    ) -> Result<Conversion, RateConverterError> {
        if matches!(amount.currency, Currency::PLN) {
            return Ok(Conversion {
                pln: amount.value,
                rate: Decimal::ONE,
                date: *at,
            });
        }

        let (rate, rate_date) = self.get(at, &amount.currency)?;

        Ok(Conversion {
            pln: rate * amount.value,
            rate,
            date: rate_date,
        })
    }

    fn get(
        &self,
        date: &NaiveDate,
        currency: &Currency,
    ) -> Result<(Decimal, NaiveDate), RateConverterError> {
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
            currency: *currency,
            date: *date,
            missing_year: self.missing_year(*date, currency),
        })
    }

    // NBP has no table on 01.01, so a lookup early in January steps back into December. When the
    // later year already has rates, the gap is in the earlier one.
    fn missing_year(&self, date: NaiveDate, currency: &Currency) -> i32 {
        let (oldest, newest) = Self::lookup_window(date);
        [newest.year(), oldest.year()]
            .into_iter()
            .find(|year| !self.has_year(*year, currency))
            .unwrap_or(newest.year())
    }

    fn has_year(&self, year: i32, currency: &Currency) -> bool {
        let (Some(start), Some(end)) = (
            NaiveDate::from_ymd_opt(year, 1, 1),
            NaiveDate::from_ymd_opt(year, 12, 31),
        ) else {
            return false;
        };
        self.rates_by_date
            .range(start..=end)
            .any(|(_, rates)| rates.contains_key(currency))
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

    fn usd_rate(date: NaiveDate, rate: Decimal) -> Rate {
        Rate {
            date,
            currency: Currency::USD,
            rate,
        }
    }

    #[test]
    fn rate_is_from_last_day_before() {
        // Friday 07.03.2025 and Monday 10.03.2025.
        let provider = NbpRateProvider::new(vec![
            usd_rate(date(2025, 3, 7), Decimal::from(4)),
            usd_rate(date(2025, 3, 10), Decimal::from(9)),
        ]);

        let monday = provider
            .convert(&usd_amount(2), &date(2025, 3, 10))
            .unwrap();
        assert_eq!(
            (monday.pln, monday.rate, monday.date),
            (Decimal::from(8), Decimal::from(4), date(2025, 3, 7))
        );

        let pln = provider
            .convert(
                &Amount {
                    value: Decimal::from(5),
                    currency: Currency::PLN,
                },
                &date(2025, 3, 10),
            )
            .unwrap();
        assert_eq!(
            (pln.pln, pln.rate, pln.date),
            (Decimal::from(5), Decimal::ONE, date(2025, 3, 10))
        );
    }

    #[test]
    fn missing_year_names_the_year_without_rates() {
        // 2025 has rates, but a lookup on 02.01 needs December's.
        let provider = NbpRateProvider::new(vec![usd_rate(date(2025, 1, 2), Decimal::ONE)]);
        let error = provider
            .convert(&usd_amount(1), &date(2025, 1, 2))
            .unwrap_err();
        assert_eq!(error.missing_year(), Some(2024));

        let provider = NbpRateProvider::new(vec![usd_rate(date(2023, 6, 1), Decimal::ONE)]);
        let error = provider
            .convert(&usd_amount(1), &date(2025, 6, 10))
            .unwrap_err();
        assert_eq!(error.missing_year(), Some(2025));
    }

    #[test]
    fn convert_returns_rate_when_lookup_takes_ten_steps() {
        let provider = NbpRateProvider::new(vec![Rate {
            date: date(2024, 1, 1),
            currency: Currency::USD,
            rate: Decimal::ONE,
        }]);

        let conversion = provider
            .convert(&usd_amount(2), &date(2024, 1, 11))
            .unwrap();

        assert_eq!(conversion.pln, Decimal::from(2));
        assert_eq!(conversion.date, date(2024, 1, 1));
    }
}
