use anyhow::Result;
use rust_decimal::Decimal;

use crate::{DecimalExt, rate::NbpRateProvider};

pub use model::{Action, CalculatedCrypto, Crypto, CryptoTaxData};

mod model;

pub fn calculate_sell_buy_values(
    cryptos: Vec<Crypto>,
    rate_provider: &NbpRateProvider,
) -> Result<CryptoTaxData> {
    let mut income = Decimal::ZERO;
    let mut costs = Decimal::ZERO;

    let mut calculated = Vec::with_capacity(cryptos.len());

    for crypto in cryptos {
        let (value_pln, nbp_date) = rate_provider.convert(&crypto.value, &crypto.date)?;
        let (fee_pln, _) = rate_provider.convert(&crypto.fee, &crypto.date)?;

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
        income: income.round_amount(),
        costs: costs.round_amount(),
        calculated,
    })
}
