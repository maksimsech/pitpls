use thiserror::Error;

mod model;

pub use model::{CalculatedInterest, Interest, InterestTaxData};
use rust_decimal::Decimal;

use crate::{
    DecimalExt,
    rate::{NbpRateProvider, RateConverterError},
    tax::POLAND_TAX,
    time::tax_date,
};

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum CalculateInterestTaxError {
    InterestConversion(#[source] RateConverterError),
}

pub fn calculate(
    interests: Vec<Interest>,
    rate_provider: &NbpRateProvider,
) -> Result<InterestTaxData, CalculateInterestTaxError> {
    let mut to_pay_total = Decimal::ZERO;
    let mut profit = Decimal::ZERO;
    let mut calculated = Vec::with_capacity(interests.len());

    for interest in interests {
        let conversion = rate_provider
            .convert(&interest.value, &tax_date(interest.time))
            .map_err(CalculateInterestTaxError::InterestConversion)?;
        let interest_pln = conversion.pln;
        let to_pay = interest_pln * POLAND_TAX;

        profit += interest_pln;
        to_pay_total += to_pay;

        calculated.push(CalculatedInterest::build(
            interest,
            conversion.date,
            conversion.rate,
            interest_pln,
            to_pay,
        ));
    }

    Ok(InterestTaxData {
        to_pay: to_pay_total.round_groszy(),
        income: profit.round_groszy(),
        calculated,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate, TimeZone, Utc};
    use rust_decimal::dec;

    use super::*;
    use crate::{
        common::{Amount, Currency},
        rate::Rate,
    };

    #[test]
    fn tax_rounds_the_sum_not_each_record() {
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
        let interest = |id: &str, day| Interest {
            id: id.to_owned(),
            time: Utc.with_ymd_and_hms(2025, 3, day, 2, 0, 0).unwrap(),
            value: Amount {
                value: dec!(0.03),
                currency: Currency::USD,
            },
            provider: "Trading 212".to_owned(),
        };

        let data = calculate(vec![interest("a", 3), interest("b", 4)], &rates).unwrap();

        // 0.0228 twice: 0.05 from the sum, 0.04 from rounded records.
        assert_eq!(data.calculated[0].to_pay, dec!(0.0228));
        assert_eq!((data.income, data.to_pay), (dec!(0.24), dec!(0.05)));
    }
}
