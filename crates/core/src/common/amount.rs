use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::currency::Currency;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Amount {
    pub value: Decimal,
    pub currency: Currency,
}
