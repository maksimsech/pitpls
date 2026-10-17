use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::common::{Amount, Country, Currency};

pub struct Position {
    pub ticker: String,
    pub provider: String,
    pub currency: Currency,
    pub shares: Decimal,
    pub avg_price: Decimal,
    pub cost: Decimal,
    pub since: DateTime<Utc>,
    pub lots: usize,
}

#[derive(Default)]
pub struct RealizedYear {
    pub year: i32,
    pub sales: usize,
    pub income: Decimal,
    pub costs: Decimal,
    pub profit: Decimal,
}

pub struct RealizedInstrument {
    pub ticker: String,
    pub provider: String,
    pub sales: usize,
    pub shares: Decimal,
    pub income: Decimal,
    pub costs: Decimal,
    pub profit: Decimal,
    pub first_sale: DateTime<Utc>,
    pub last_sale: DateTime<Utc>,
    pub held: bool,
}

#[derive(Default)]
pub struct DividendYear {
    pub year: i32,
    pub payments: usize,
    pub payers: usize,
    pub gross: Decimal,
    pub withheld: Decimal,
    pub net: Decimal,
}

pub struct DividendPayer {
    pub ticker: String,
    pub provider: String,
    pub country: Country,
    pub payments: usize,
    pub first_year: i32,
    pub last_year: i32,
    pub gross_in_currency: Vec<Amount>,
    pub gross: Decimal,
    pub withheld: Decimal,
    pub net: Decimal,
}

#[derive(Default)]
pub struct InterestMonth {
    pub year: i32,
    pub month: u32,
    pub records: usize,
    pub value: Decimal,
    pub providers: Vec<(String, usize)>,
}

#[derive(Default)]
pub struct InterestYear {
    pub year: i32,
    pub records: usize,
    pub by_provider: Vec<(String, Decimal)>,
    pub total: Decimal,
}

pub struct InterestProvider {
    pub provider: String,
    pub first: DateTime<Utc>,
    pub last: DateTime<Utc>,
    pub records: usize,
    pub received_in_currency: Vec<Amount>,
    pub received: Decimal,
}

pub struct Last12Months {
    pub value: Decimal,
    pub from: (i32, u32),
    pub to: (i32, u32),
}

#[derive(Default)]
pub struct CryptoYear {
    pub year: i32,
    pub buys: usize,
    pub bought: Decimal,
    pub sells: usize,
    pub sold: Decimal,
    pub fees: Decimal,
    pub net: Decimal,
}

pub struct CryptoTotals {
    pub put_in: Decimal,
    pub taken_out: Decimal,
    pub net: Decimal,
}

#[derive(Default)]
pub struct EarnedYear {
    pub year: i32,
    pub realized: Decimal,
    pub dividends: Decimal,
    pub interest: Decimal,
    pub total: Decimal,
}
