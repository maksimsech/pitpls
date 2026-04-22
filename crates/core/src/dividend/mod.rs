use anyhow::Result;
use rust_decimal::Decimal;

use crate::{
    DecimalExt,
    rate::NbpRateProvider,
    tax::{POLAND_TAX, get_treaty_tax},
};

mod model;

pub use model::{CalculatedDividend, Dividend, DividendTaxData};

pub fn calculate(
    dividends: Vec<Dividend>,
    rate_provider: &NbpRateProvider,
) -> Result<DividendTaxData> {
    let mut to_pay_total = Decimal::ZERO;
    let mut paid_total = Decimal::ZERO;
    let mut profit = Decimal::ZERO;
    let mut calculated = Vec::with_capacity(dividends.len());

    for dividend in dividends {
        let (dividend_pln, nbp_date) = rate_provider.convert(&dividend.value, &dividend.date)?;
        let to_pay = dividend_pln * POLAND_TAX;

        profit += dividend_pln;
        to_pay_total += to_pay;

        let AlreadyPaidData {
            calculated_tax_paid,
            max_tax_paid,
            used_tax_paid,
        } = calculate_already_paid(&dividend, dividend_pln, rate_provider)?;
        paid_total += used_tax_paid;

        calculated.push(CalculatedDividend::build(
            dividend,
            nbp_date,
            dividend_pln,
            to_pay,
            calculated_tax_paid,
            max_tax_paid,
            used_tax_paid,
        ));
    }

    Ok(DividendTaxData {
        to_pay: to_pay_total.round_amount(),
        paid: paid_total.round_amount(),
        profit: profit.round_amount(),
        calculated,
    })
}

fn calculate_already_paid(
    dividend: &Dividend,
    dividend_pln: Decimal,
    rate_provider: &NbpRateProvider,
) -> Result<AlreadyPaidData> {
    let (paid_pln, _) = rate_provider.convert(&dividend.tax_paid, &dividend.date)?;

    let max_paid_pln = get_treaty_tax(&dividend.country) * dividend_pln;

    let used_tax_paid = if paid_pln > max_paid_pln {
        max_paid_pln
    } else {
        paid_pln
    };

    Ok(AlreadyPaidData {
        calculated_tax_paid: paid_pln,
        max_tax_paid: max_paid_pln,
        used_tax_paid,
    })
}

struct AlreadyPaidData {
    pub calculated_tax_paid: Decimal,
    pub max_tax_paid: Decimal,
    pub used_tax_paid: Decimal,
}
