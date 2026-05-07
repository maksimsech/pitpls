use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    DecimalExt,
    rate::{NbpRateProvider, RateConverterError},
};

pub use model::{Action, CalculatedCrypto, Crypto, CryptoTaxData};

mod model;

#[derive(Debug, Error)]
pub enum CalculateSellBuyValuesError {
    #[error("Failed to convert crypto value to PLN: {0}")]
    ValueConversion(#[source] RateConverterError),
    #[error("Failed to convert crypto fee to PLN: {0}")]
    FeeConversion(#[source] RateConverterError),
}

pub fn calculate_sell_buy_values(
    cryptos: Vec<Crypto>,
    rate_provider: &NbpRateProvider,
) -> Result<CryptoTaxData, CalculateSellBuyValuesError> {
    let mut income = Decimal::ZERO;
    let mut costs = Decimal::ZERO;

    let mut calculated = Vec::with_capacity(cryptos.len());

    for crypto in cryptos {
        let (value_pln, nbp_date) = rate_provider
            .convert(&crypto.value, &crypto.date)
            .map_err(CalculateSellBuyValuesError::ValueConversion)?;
        let (fee_pln, _) = rate_provider
            .convert(&crypto.fee, &crypto.date)
            .map_err(CalculateSellBuyValuesError::FeeConversion)?;

        match crypto.action {
            Action::FiatBuy => {
                costs += value_pln + fee_pln;
            }
            Action::FiatSell => {
                income += value_pln;
                costs += fee_pln;
            }
        }

        calculated.push(CalculatedCrypto::build(
            crypto, value_pln, fee_pln, nbp_date,
        ));
    }

    Ok(CryptoTaxData {
        income: income.round_groszy(),
        costs: costs.round_groszy(),
        calculated,
    })
}
