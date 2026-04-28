use rust_decimal::{Decimal, dec};

use crate::common::Country;

pub const POLAND_TAX: Decimal = dec!(0.19);

pub fn get_treaty_tax(country: &Country) -> Decimal {
    match country {
        Country::Japan => dec!(0.10),
        Country::USA => dec!(0.15),
        Country::Other(_) => Decimal::ZERO,
    }
}
