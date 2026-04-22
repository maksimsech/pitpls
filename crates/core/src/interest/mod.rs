use anyhow::Result;

mod model;

pub use model::{CalculatedInterest, Interest, InterestTaxData};
use rust_decimal::Decimal;

use crate::{DecimalExt, rate::NbpRateProvider, tax::POLAND_TAX};

pub fn calculate(
    interests: Vec<Interest>,
    rate_provider: &NbpRateProvider,
) -> Result<InterestTaxData> {
    let mut to_pay_total = Decimal::ZERO;
    let mut profit = Decimal::ZERO;
    let mut calculated = Vec::with_capacity(interests.len());

    for interest in interests {
        let (interest_pln, nbp_date) = rate_provider.convert(&interest.value, &interest.date)?;
        let to_pay = interest_pln * POLAND_TAX;

        profit += interest_pln;
        to_pay_total += to_pay;

        calculated.push(CalculatedInterest::build(interest, nbp_date, to_pay));
    }

    Ok(InterestTaxData {
        to_pay: to_pay_total.round_amount(),
        profit: profit.round_amount(),
        calculated,
    })
}
