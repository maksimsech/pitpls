use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::common::Amount;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    FiatBuy,
    FiatSell,
}

pub struct Crypto {
    pub id: String,
    pub value: Amount,
    pub fee: Amount,
    pub action: Action,
    pub date: NaiveDate,
    pub provider: String,
}

pub struct CalculatedCrypto {
    pub id: String,
    pub value: Amount,
    pub calculated_value: Decimal,
    pub fee: Amount,
    pub calculated_fee: Decimal,
    pub action: Action,
    pub date: NaiveDate,
    pub nbp_date: NaiveDate,
    // The NBP rate of `nbp_date` for the value's currency (1 for PLN).
    pub nbp_rate: Decimal,
    // The NBP rate for the fee, whose currency can differ (1 for PLN).
    pub fee_nbp_rate: Decimal,
    pub provider: String,
}

impl CalculatedCrypto {
    pub fn build(
        crypto: Crypto,
        calculated_value: Decimal,
        calculated_fee: Decimal,
        nbp_date: NaiveDate,
        nbp_rate: Decimal,
        fee_nbp_rate: Decimal,
    ) -> Self {
        Self {
            id: crypto.id,
            value: crypto.value,
            calculated_value,
            fee: crypto.fee,
            calculated_fee,
            action: crypto.action,
            date: crypto.date,
            nbp_date,
            nbp_rate,
            fee_nbp_rate,
            provider: crypto.provider,
        }
    }
}

pub struct CryptoTaxData {
    pub income: Decimal,
    pub costs: Decimal,
    pub calculated: Vec<CalculatedCrypto>,
}
