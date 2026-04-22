use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Serialize, Type)]
pub struct CryptoTaxSummary {
    pub income: Decimal,
    pub costs: Decimal,
}

#[derive(Debug, Serialize, Type)]
pub struct ForeignTaxSummary {
    pub income: Decimal,
    pub tax_to_pay: Decimal,
    pub tax_paid: Decimal,
}

#[derive(Debug, Serialize, Type)]
pub struct TaxSummary {
    pub crypto: CryptoTaxSummary,
    pub foreign: ForeignTaxSummary,
}
