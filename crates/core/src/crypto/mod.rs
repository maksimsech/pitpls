use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    DecimalExt,
    rate::{NbpRateProvider, RateConverterError},
    time::tax_date,
};

pub use model::{Action, CalculatedCrypto, Crypto, CryptoTaxData};

mod model;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum CalculateSellBuyValuesError {
    ValueConversion(#[source] RateConverterError),
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
        let date = tax_date(crypto.time);
        let value = rate_provider
            .convert(&crypto.value, &date)
            .map_err(CalculateSellBuyValuesError::ValueConversion)?;
        let fee = rate_provider
            .convert(&crypto.fee, &date)
            .map_err(CalculateSellBuyValuesError::FeeConversion)?;
        let (value_pln, fee_pln) = (value.pln, fee.pln);

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
            crypto, value_pln, fee_pln, value.date, value.rate, fee.rate,
        ));
    }

    Ok(CryptoTaxData {
        income: income.round_groszy(),
        costs: costs.round_groszy(),
        calculated,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate, TimeZone, Utc};
    use rust_decimal::dec;

    use super::*;
    use crate::{
        common::{Amount, Currency},
        rate::Rate,
    };

    fn eur(value: Decimal) -> Amount {
        Amount {
            value,
            currency: Currency::EUR,
        }
    }

    #[test]
    fn buy_costs_value_and_fee_sale_counts_value_as_income() {
        let rates = NbpRateProvider::new(
            NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .iter_days()
                .take_while(|day| day.year() == 2025)
                .map(|date| Rate {
                    date,
                    currency: Currency::EUR,
                    rate: dec!(4.5),
                })
                .collect(),
        );
        let crypto = |id: &str, action, value, fee| Crypto {
            id: id.to_owned(),
            value: eur(value),
            fee: eur(fee),
            action,
            time: Utc.with_ymd_and_hms(2025, 4, 9, 8, 0, 0).unwrap(),
            provider: "Coinbase".to_owned(),
        };
        let cryptos = vec![
            crypto("buy", Action::FiatBuy, dec!(100), dec!(1)),
            crypto("sell", Action::FiatSell, dec!(200), dec!(2)),
        ];

        let data = calculate_sell_buy_values(cryptos, &rates).unwrap();

        assert_eq!((data.income, data.costs), (dec!(900), dec!(463.5)));
        let sell = &data.calculated[1];
        assert_eq!(
            (sell.calculated_value, sell.calculated_fee, sell.nbp_rate),
            (dec!(900), dec!(9), dec!(4.5))
        );
    }
}
