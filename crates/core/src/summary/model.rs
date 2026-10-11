use rust_decimal::Decimal;

use crate::stock::{CountryTotal, Warning};

#[derive(Debug)]
pub struct CryptoTaxSummary {
    pub income: Decimal,
    pub costs: Decimal,
}

#[derive(Debug)]
pub struct ForeignTaxSummary {
    pub income: Decimal,
    pub tax_to_pay: Decimal,
    pub tax_paid: Decimal,
}

#[derive(Debug)]
pub struct StockTaxSummary {
    pub income: Decimal,
    pub costs: Decimal,
    pub tax_paid_abroad: Decimal,
    // Loss countries too, so Summary can say they need no PIT/ZG.
    pub countries: Vec<CountryTotal>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug)]
pub struct TaxSummary {
    pub crypto: CryptoTaxSummary,
    pub foreign: ForeignTaxSummary,
    pub stocks: StockTaxSummary,
}
