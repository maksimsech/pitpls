use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::common::Currency;

pub struct Rate {
    pub date: NaiveDate,
    pub currency: Currency,
    pub rate: Decimal,
}
