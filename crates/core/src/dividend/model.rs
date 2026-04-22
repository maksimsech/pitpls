use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;

use crate::common::{Amount, Country};

pub struct Dividend {
    pub id: String,
    pub date: NaiveDate,
    pub ticker: String,
    pub value: Amount,
    pub tax_paid: Amount,
    pub country: Country,
    pub provider: String,
}

#[derive(Serialize, Type)]
pub struct CalculatedDividend {
    pub id: String,
    pub date: NaiveDate,
    pub nbp_date: NaiveDate,
    pub ticker: String,
    pub value: Amount,
    pub calculated_value: Decimal,
    pub calculated_to_pay: Decimal,
    pub tax_paid: Amount,
    pub calculated_tax_paid: Decimal,
    pub max_tax_paid: Decimal,
    pub used_tax_paid: Decimal,
    pub country: Country,
    pub provider: String,
}

impl CalculatedDividend {
    pub fn build(
        dividend: Dividend,
        nbp_date: NaiveDate,
        calculated_value: Decimal,
        calculated_to_pay: Decimal,
        calculated_tax_paid: Decimal,
        max_tax_paid: Decimal,
        used_tax_paid: Decimal,
    ) -> Self {
        CalculatedDividend {
            id: dividend.id,
            date: dividend.date,
            nbp_date,
            ticker: dividend.ticker,
            value: dividend.value,
            calculated_value,
            calculated_to_pay,
            tax_paid: dividend.tax_paid,
            calculated_tax_paid,
            max_tax_paid,
            used_tax_paid,
            country: dividend.country,
            provider: dividend.provider,
        }
    }
}

#[derive(Serialize, Type)]
pub struct DividendTaxData {
    pub to_pay: Decimal,
    pub paid: Decimal,
    pub income: Decimal,
    pub calculated: Vec<CalculatedDividend>,
}
