use std::{collections::HashMap, vec};

use anyhow::Result;
use chrono::Datelike;
use rust_decimal::Decimal;

use crate::{
    common::Amount,
    rate::NbpRateProvider,
    stock::model::{CalculatedStock, StockTaxData},
};

mod model;

pub use model::{Stock, StockAction, StockBuy, StockSell, StockSplit};

/*
 * For now just accept full stock data, later I could optimize it.
 * Method assumes that data as loaded for a >= year
 */
pub fn calculate_sell_buy_values(
    stocks: Vec<Stock>,
    year: i32,
    rate_provider: &NbpRateProvider,
) -> Result<StockTaxData> {
    debug_assert!(stocks.iter().all(|s| s.date.year() <= year));

    let this_year_sells = stocks.iter().filter(|s| s.date.year() == year);
    let mut this_year_sells_map = HashMap::new();
    for sell in this_year_sells {
        if let StockAction::Sell(ref sell_action) = sell.action {
            this_year_sells_map
                .entry((sell.ticker.as_str(), sell_action.provider.as_str()))
                .and_modify(|v: &mut Vec<&Stock>| v.push(sell))
                .or_insert_with(|| vec![sell]);
        }
    }

    let mut calculated_stocks = vec![];
    for stock in this_year_sells_map {
        let ticker = stock.0.0.to_owned();
        let provider = stock.0.1.to_owned();
        let calculated_stock =
            calculate_stock(ticker, provider, year, &stock.1, &stocks, rate_provider)?;

        calculated_stocks.push(calculated_stock);
    }
    Ok(StockTaxData {
        stocks: calculated_stocks,
    })
}

fn calculate_stock(
    ticker: String,
    provider: String,
    year: i32,
    this_year_sells: &[&Stock],
    all_stocks: &[Stock],
    rate_provider: &NbpRateProvider,
) -> Result<CalculatedStock> {
    let mut stock_history = all_stocks
        .iter()
        .filter(|s| {
            let same_ticker = s.ticker == ticker;
            if !same_ticker {
                return false;
            }

            match s.action {
                StockAction::Split(_) => true,
                StockAction::Buy(ref b) if b.provider == provider => true,
                StockAction::Sell(ref b) if b.provider == provider /*&& s.date.year() != year */ => true,
                _ => false,
            }
        })
        .collect::<Vec<_>>();

    stock_history.sort_unstable_by_key(|s| s.date);

    /*
     * TODO: What I need to do there:
     * Replay all the events from start. With calculating information on how much I sold. How much I have to pay.
     * Year by year. I might have a cases when stock was sold but, where were not enough bought amount,
     * this should not be an error, but a warning with detailed information on how it happen.
     *
     * For a specific year I have effectively two fazes:
     * 1. Everything before current year: I need to replay it to get *current situation*: what I have and cost basis for this.
     * 2. Current year: aside for replaying as usual I need to calculate taxes: all sells are as a side effect produce tax information.
     *
     * Generally speaking both steps are the same: only difference that I only care about taxes for specific year.
     * As I simplification I can write method to always calculate taxes, and later optimize, or leave as is, bcs maybe this information will be usable.
     */

    let mut calculated_stock = CalculatedStock {
        ticker,
        provider,
        bought: 0.into(),
        sold: 0.into(),
        value: 0.into(),
        warnings: vec![],
        history: vec![],
    };

    Ok(calculated_stock)
}
