use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    DecimalExt,
    rate::{NbpRateProvider, RateConverterError},
    settings::DividendRounding,
    tax::{POLAND_TAX, get_treaty_tax},
};

mod model;

pub use model::{CalculatedDividend, Dividend, DividendTaxData};

#[derive(Debug, Error)]
pub enum CalculateDividendTaxError {
    #[error("Failed to convert dividend value to PLN: {0}")]
    DividendConversion(#[source] RateConverterError),
    #[error("Failed to convert paid dividend tax to PLN: {0}")]
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
            .convert(&dividend.value, &dividend.date)
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
        .convert(&dividend.tax_paid, &dividend.date)
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
