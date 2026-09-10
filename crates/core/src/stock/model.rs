use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::common::Amount;

pub struct Stock {
    pub id: String,
    pub date: NaiveDate,
    pub ticker: String,
    pub action: StockAction,
}

pub enum StockAction {
    Buy(StockBuy),
    Sell(StockSell),
    Split(StockSplit),
}

pub struct StockBuy {
    pub number: Decimal,
    pub price: Amount,
    pub fee: Amount,
    pub provider: String,
}

pub struct StockSell {
    pub number: Decimal,
    pub price: Amount,
    pub fee: Amount,
    pub provider: String,
}

pub struct StockSplit {
    pub ratio: Decimal,
}
