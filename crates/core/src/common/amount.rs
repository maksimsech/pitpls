use rust_decimal::Decimal;

use super::currency::Currency;

#[derive(Clone, Copy)]
pub struct Amount {
    pub value: Decimal,
    pub currency: Currency,
}
