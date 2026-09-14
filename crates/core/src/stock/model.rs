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
    pub bought: Decimal,
    pub sold: Decimal,
    pub value: Decimal,
    // TODO: Should be enum later,
    pub warnings: Vec<String>,
    // TODO: As an option I could use different contract to lower memory used
    pub history: Vec<Stock>,
}
