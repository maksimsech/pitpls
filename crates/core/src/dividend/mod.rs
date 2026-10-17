use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    DecimalExt,
    rate::{NbpRateProvider, RateConverterError},
    settings::DividendRounding,
    tax::{POLAND_TAX, get_treaty_tax},
    time::tax_date,
};

mod model;

pub use model::{CalculatedDividend, Dividend, DividendTaxData};

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum CalculateDividendTaxError {
    DividendConversion(#[source] RateConverterError),
    PaidTaxConversion(#[source] RateConverterError),
}

pub fn calculate(
    dividends: Vec<Dividend>,
    rate_provider: &NbpRateProvider,
    rounding: DividendRounding,
) -> Result<DividendTaxData, CalculateDividendTaxError> {
    let mut to_pay_total = Decimal::ZERO;
    let mut paid_total = Decimal::ZERO;
    let mut profit = Decimal::ZERO;
    let mut calculated = Vec::with_capacity(dividends.len());

    for dividend in dividends {
        let conversion = rate_provider
            .convert(&dividend.value, &tax_date(dividend.time))
            .map_err(CalculateDividendTaxError::DividendConversion)?;
        let dividend_pln = conversion.pln.maybe_round_dividend(rounding);

        let mut to_pay = dividend_pln * POLAND_TAX;
        to_pay = to_pay.maybe_round_dividend(rounding);

        profit += dividend_pln;
        to_pay_total += to_pay;

        let AlreadyPaidData {
            nbp_rate: tax_paid_nbp_rate,
            calculated_tax_paid,
            max_tax_paid,
            used_tax_paid,
        } = calculate_already_paid(&dividend, dividend_pln, rate_provider, rounding)?;
        paid_total += used_tax_paid;

        calculated.push(CalculatedDividend::build(
            dividend,
            conversion.date,
            conversion.rate,
            tax_paid_nbp_rate,
            dividend_pln,
            to_pay,
            calculated_tax_paid,
            max_tax_paid,
            used_tax_paid,
        ));
    }

    Ok(DividendTaxData {
        to_pay: if matches!(
            rounding,
            DividendRounding::SumToPayToZlote | DividendRounding::SumBothToZlote
        ) {
            to_pay_total.round_zloty()
        } else {
            to_pay_total.round_groszy()
        },
        paid: if matches!(rounding, DividendRounding::SumBothToZlote) {
            paid_total.round_zloty()
        } else {
            paid_total.round_groszy()
        },
        income: profit.round_groszy(),
        calculated,
    })
}

fn calculate_already_paid(
    dividend: &Dividend,
    dividend_pln: Decimal,
    rate_provider: &NbpRateProvider,
    rounding: DividendRounding,
) -> Result<AlreadyPaidData, CalculateDividendTaxError> {
    let conversion = rate_provider
        .convert(&dividend.tax_paid, &tax_date(dividend.time))
        .map_err(CalculateDividendTaxError::PaidTaxConversion)?;
    let paid_pln = conversion.pln.maybe_round_dividend(rounding);

    let mut max_paid_pln = get_treaty_tax(&dividend.country) * dividend_pln;
    max_paid_pln = max_paid_pln.maybe_round_dividend(rounding);

    let used_tax_paid = if paid_pln > max_paid_pln {
        max_paid_pln
    } else {
        paid_pln
    };

    Ok(AlreadyPaidData {
        nbp_rate: conversion.rate,
        calculated_tax_paid: paid_pln,
        max_tax_paid: max_paid_pln,
        used_tax_paid,
    })
}

struct AlreadyPaidData {
    pub nbp_rate: Decimal,
    pub calculated_tax_paid: Decimal,
    pub max_tax_paid: Decimal,
    pub used_tax_paid: Decimal,
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
    use rust_decimal::dec;

    use super::*;
    use crate::{
        common::{Amount, Country, Currency},
        rate::Rate,
    };

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn on(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
    }

    fn usd(value: Decimal) -> Amount {
        Amount {
            value,
            currency: Currency::USD,
        }
    }

    // The same rates on every day of 2025.
    fn flat(rates: &[(Currency, Decimal)]) -> NbpRateProvider {
        NbpRateProvider::new(
            date(2025, 1, 1)
                .iter_days()
                .take_while(|day| day.year() == 2025)
                .flat_map(|date| {
                    rates.iter().map(move |&(currency, rate)| Rate {
                        date,
                        currency,
                        rate,
                    })
                })
                .collect(),
        )
    }

    fn dividend(id: &str, country: Country, value: Amount, tax_paid: Amount) -> Dividend {
        Dividend {
            id: id.to_owned(),
            time: on(2025, 5, 15),
            ticker: id.to_uppercase(),
            value,
            tax_paid,
            country,
            provider: "Trading 212".to_owned(),
        }
    }

    fn found<'a>(data: &'a DividendTaxData, id: &str) -> &'a CalculatedDividend {
        data.calculated
            .iter()
            .find(|dividend| dividend.id == id)
            .unwrap_or_else(|| panic!("no dividend {id}"))
    }

    #[test]
    fn tax_is_19_percent_less_credit_capped_by_treaty() {
        let germany = "DE".parse().unwrap();
        let dividends = vec![
            dividend("us", Country::USA, usd(dec!(100)), usd(dec!(15))),
            dividend("us-over", Country::USA, usd(dec!(100)), usd(dec!(30))),
            dividend("jp", Country::Japan, usd(dec!(100)), usd(dec!(15))),
            dividend("de", germany, usd(dec!(100)), usd(dec!(26.375))),
        ];

        let data = calculate(
            dividends,
            &flat(&[(Currency::USD, dec!(4))]),
            DividendRounding::SumToGroszy,
        )
        .unwrap();

        let credit = |id: &str| {
            let dividend = found(&data, id);
            (
                dividend.calculated_value,
                dividend.calculated_to_pay,
                dividend.calculated_tax_paid,
                dividend.max_tax_paid,
                dividend.used_tax_paid,
            )
        };
        assert_eq!(
            credit("us"),
            (dec!(400), dec!(76), dec!(60), dec!(60), dec!(60))
        );
        assert_eq!(
            credit("us-over"),
            (dec!(400), dec!(76), dec!(120), dec!(60), dec!(60))
        );
        assert_eq!(
            credit("jp"),
            (dec!(400), dec!(76), dec!(60), dec!(40), dec!(40))
        );
        assert_eq!(
            credit("de"),
            (dec!(400), dec!(76), dec!(105.5), dec!(0), dec!(0))
        );
        assert_eq!(
            (data.income, data.to_pay, data.paid),
            (dec!(1600), dec!(304), dec!(160))
        );
    }

    #[test]
    fn rounding_setting_rounds_records_or_totals() {
        // 41.48 PLN and 150 PLN; 19% of 150 and its 15% credit end in .5.
        let dividends = || {
            vec![
                dividend("a", Country::USA, usd(dec!(10.37)), usd(dec!(1.55))),
                dividend("b", Country::USA, usd(dec!(37.5)), usd(dec!(5.625))),
            ]
        };
        let rates = flat(&[(Currency::USD, dec!(4))]);
        let totals = |rounding| {
            let data = calculate(dividends(), &rates, rounding).unwrap();
            (data.income, data.to_pay, data.paid)
        };

        assert_eq!(
            totals(DividendRounding::SumToGroszy),
            (dec!(191.48), dec!(36.38), dec!(28.70))
        );
        assert_eq!(
            totals(DividendRounding::SumToPayToZlote),
            (dec!(191.48), dec!(36), dec!(28.70))
        );
        assert_eq!(
            totals(DividendRounding::SumBothToZlote),
            (dec!(191.48), dec!(36), dec!(29))
        );
        assert_eq!(
            totals(DividendRounding::AllToZlote),
            (dec!(191), dec!(37), dec!(29))
        );

        let data = calculate(dividends(), &rates, DividendRounding::AllToZlote).unwrap();
        let b = found(&data, "b");
        assert_eq!(
            (b.calculated_to_pay, b.calculated_tax_paid, b.used_tax_paid),
            (dec!(29), dec!(23), dec!(23))
        );
    }

    #[test]
    fn tax_paid_uses_its_own_currency_rate() {
        let tax_paid = Amount {
            value: dec!(10),
            currency: Currency::EUR,
        };
        let dividends = vec![dividend("a", Country::USA, usd(dec!(100)), tax_paid)];
        let rates = flat(&[(Currency::USD, dec!(4)), (Currency::EUR, dec!(4.5))]);

        let data = calculate(dividends, &rates, DividendRounding::SumToGroszy).unwrap();
        let a = found(&data, "a");

        assert_eq!((a.nbp_rate, a.tax_paid_nbp_rate), (dec!(4), dec!(4.5)));
        assert_eq!(
            (a.calculated_tax_paid, a.used_tax_paid),
            (dec!(45), dec!(45))
        );
    }

    #[test]
    fn rate_comes_from_the_day_before_the_warsaw_day() {
        let rates = NbpRateProvider::new(vec![
            Rate {
                date: date(2025, 12, 30),
                currency: Currency::USD,
                rate: dec!(4),
            },
            Rate {
                date: date(2025, 12, 31),
                currency: Currency::USD,
                rate: dec!(3.5),
            },
        ]);
        let mut late = dividend("late", Country::USA, usd(dec!(10)), usd(dec!(0)));
        // 01.01.2026 in Warsaw.
        late.time = Utc.with_ymd_and_hms(2025, 12, 31, 23, 30, 0).unwrap();

        let data = calculate(vec![late], &rates, DividendRounding::SumToGroszy).unwrap();
        let late = found(&data, "late");

        assert_eq!(
            (late.nbp_date, late.nbp_rate, late.calculated_value),
            (date(2025, 12, 31), dec!(3.5), dec!(35))
        );
    }
}
