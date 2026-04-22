use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::common::Amount;

#[derive(Clone, Copy, Serialize, Deserialize, Type)]
pub enum Action {
    FiatBuy,
    FiatSell,
}

#[derive(Clone)]
pub struct Crypto {
    pub id: String,
    pub value: Amount,
    pub fee: Amount,
    pub action: Action,
    pub date: NaiveDate,
    pub provider: String,
}

#[derive(Serialize, Type)]
pub struct CalculatedCrypto {
    pub id: String,
    pub value: Amount,
    pub calculated_value: Decimal,
    pub fee: Amount,
    pub calculated_fee: Decimal,
    pub action: Action,
    pub date: NaiveDate,
    pub nbp_date: NaiveDate,
    pub provider: String,
}

impl CalculatedCrypto {
    pub fn build(
        crypto: Crypto,
        calculated_value: Decimal,
        calculated_fee: Decimal,
        nbp_date: NaiveDate,
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
            provider: crypto.provider,
        }
    }
}

#[derive(Serialize, Type)]
pub struct CryptoTaxData {
    pub income: Decimal,
    pub costs: Decimal,
    pub calculated: Vec<CalculatedCrypto>,
}
