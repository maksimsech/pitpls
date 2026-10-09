use thiserror::Error;

mod model;

pub use model::{CalculatedInterest, Interest, InterestTaxData};
use rust_decimal::Decimal;

use crate::{
    DecimalExt,
    rate::{NbpRateProvider, RateConverterError},
    tax::POLAND_TAX,
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
            .convert(&interest.value, &interest.date)
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
