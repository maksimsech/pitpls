use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CryptoTaxSummary {
    pub income: Decimal,
    pub costs: Decimal,
}

#[derive(Debug, Serialize)]
pub struct ForeignTaxSummary {
    pub income: Decimal,
    pub tax_to_pay: Decimal,
    pub tax_paid: Decimal,
}

#[derive(Debug, Serialize)]
pub struct TaxSummary {
    pub crypto: CryptoTaxSummary,
    pub foreign: ForeignTaxSummary,
}
