use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;

use crate::common::Amount;

pub struct Interest {
    pub id: String,
    pub date: NaiveDate,
    pub value: Amount,
    pub provider: String,
}

#[derive(Serialize, Type)]
pub struct CalculatedInterest {
    pub id: String,
    pub date: NaiveDate,
    pub nbp_date: NaiveDate,
    pub value: Amount,
    pub calculated_value: Decimal,
    pub to_pay: Decimal,
    pub provider: String,
}

impl CalculatedInterest {
    pub fn build(
        interest: Interest,
        nbp_date: NaiveDate,
        calculated_value: Decimal,
        to_pay: Decimal,
    ) -> Self {
        Self {
            id: interest.id,
            date: interest.date,
            nbp_date,
            value: interest.value,
            calculated_value,
            to_pay,
            provider: interest.provider,
        }
    }
}

#[derive(Serialize, Type)]
pub struct InterestTaxData {
    pub to_pay: Decimal,
    pub income: Decimal,
    pub calculated: Vec<CalculatedInterest>,
}
