use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use crate::common::{Amount, Mic};

#[derive(Clone)]
pub struct Stock {
    pub id: String,
    pub time: DateTime<Utc>,
    pub ticker: String,
    pub action: StockAction,
}

#[derive(Clone)]
pub enum StockAction {
    Buy(StockBuy),
    Sell(StockSell),
    Split(StockSplit),
}

#[derive(Clone)]
pub struct StockBuy {
    pub number: Decimal,
    pub price: Amount,
    pub fee: Amount,
    pub provider: String,
    pub settlement_date: Option<NaiveDate>,
}

#[derive(Clone)]
pub struct StockSell {
    pub number: Decimal,
    pub price: Amount,
    pub fee: Amount,
    pub provider: String,
    pub settlement_date: Option<NaiveDate>,
    pub tax_paid: Option<Amount>,
}

// New shares = shares × after ÷ before; a ratio can't hold 1-for-3 exactly.
#[derive(Clone)]
pub struct StockSplit {
    pub before: u32,
    pub after: u32,
}

#[derive(Clone)]
pub struct Instrument {
    pub ticker: String,
    pub provider: String,
    pub exchange: Option<Mic>,
    pub isin: Option<String>,
}

pub struct StockTaxData {
    pub stocks: Vec<CalculatedStock>,
}

pub struct CalculatedStock {
    pub ticker: String,
    pub provider: String,
    pub stats: YearStats,
    pub history_by_year: HashMap<i32, YearStats>,
}

#[derive(Default)]
pub struct YearStats {
    pub bought: Decimal,
    pub sold: Decimal,
    pub tax: Decimal,
    pub warnings: Vec<Warning>,
    pub history: Vec<Stock>,
}

pub enum Warning {
    SellWithoutBuy { sell_id: String },
}
