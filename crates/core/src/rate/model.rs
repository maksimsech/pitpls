use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::common::Currency;

pub struct Rate {
    pub date: NaiveDate,
    pub currency: Currency,
    pub rate: Decimal,
}

/// An amount converted to PLN with the NBP rate of `date`, the last day with a
/// rate before the amount's own date. PLN amounts keep their value, with rate 1
/// and their own date.
#[derive(Debug)]
pub struct Conversion {
    pub pln: Decimal,
    pub rate: Decimal,
    pub date: NaiveDate,
}
