use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::common::Currency;

pub struct RateExport {
    pub date: NaiveDate,
    pub currency: Currency,
    pub rate: Decimal,
}
