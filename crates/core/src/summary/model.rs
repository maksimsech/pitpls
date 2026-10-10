use rust_decimal::Decimal;

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
pub struct TaxSummary {
    pub crypto: CryptoTaxSummary,
    pub foreign: ForeignTaxSummary,
}
