use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::common::Amount;

#[derive(Clone)]
pub struct Stock {
    pub id: String,
    pub date: NaiveDate,
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
}

#[derive(Clone)]
pub struct StockSell {
    pub number: Decimal,
    pub price: Amount,
    pub fee: Amount,
    pub provider: String,
}

#[derive(Clone)]
pub struct StockSplit {
    pub ratio: Decimal,
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
