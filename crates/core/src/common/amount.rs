use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::currency::Currency;

#[derive(Clone, Copy, Type, Deserialize, Serialize)]
pub struct Amount {
    pub value: Decimal,
    pub currency: Currency,
}
