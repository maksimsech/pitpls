use std::{
    collections::{HashMap, HashSet, LinkedList},
    vec,
};

use anyhow::Result;
use chrono::Datelike;
use rust_decimal::Decimal;

use crate::{
    rate::NbpRateProvider,
    stock::model::{CalculatedStock, StockTaxData, Warning, YearStats},
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
    let mut this_year_sells_set = HashSet::new();
    for sell in this_year_sells {
        if let StockAction::Sell(ref sell_action) = sell.action {
            this_year_sells_set.insert((sell.ticker.as_str(), sell_action.provider.as_str()));
        }
    }

    let mut calculated_stocks = Vec::with_capacity(this_year_sells_set.len());
    for stock in this_year_sells_set {
        let ticker = stock.0.to_owned();
        let provider = stock.1.to_owned();
        let calculated_stock = calculate_stock(ticker, provider, year, &stocks, rate_provider)?;

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
                StockAction::Sell(ref b) if b.provider == provider => true,
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

    let mut current_buys = LinkedList::new();
    let mut statistics_by_year: HashMap<i32, YearStats> = HashMap::new();

    for stock in stock_history {
        let year = stock.date.year();
        let year_statistics = statistics_by_year.entry(year).or_default();
        match stock.action {
            StockAction::Buy(ref b) => {
                // TODO: Check how to implement this without clone
                current_buys.push_front((&stock.date, b.clone()));
                year_statistics.bought += b.number;
            }
            StockAction::Split(ref s) => {
                for buy in current_buys.iter_mut() {
                    buy.1.number *= s.ratio;
                }
                year_statistics.bought *= s.ratio;
            }
            StockAction::Sell(ref s) => {
                year_statistics.sold += s.number;
                let mut to_sell = s.number;

                while to_sell > Decimal::ZERO {
                    let buy_maybe = current_buys.front_mut();
                    let mut pop_buy = false;
                    match buy_maybe {
                        // TODO: Simplify
                        Some(buy) => {
                            let left = buy.1.number - to_sell;
                            let sold = if left < Decimal::ZERO {
                                buy.1.number
                            } else {
                                to_sell
                            };

                            buy.1.number -= sold;

                            let (buy_price, _) = rate_provider.convert(&buy.1.price, buy.0)?;
                            let (buy_fee, _) = rate_provider.convert(&buy.1.fee, buy.0)?;
                            let (sell_price, _) = rate_provider.convert(&s.price, &stock.date)?;
                            let (sell_fee, _) = rate_provider.convert(&s.fee, &stock.date)?;
                            year_statistics.tax +=
                                buy_price * sold - sell_price * sell_price - buy_fee - sell_fee;

                            if left <= Decimal::ZERO {
                                pop_buy = true;
                            }

                            to_sell -= sold;
                            debug_assert!(to_sell >= Decimal::ZERO);
                        }
                        // TODO: No buy left to cover. Show warning and calculate with cost basis as 0
                        None => {
                            year_statistics.warnings.push(Warning::SellWithoutBuy {
                                sell_id: stock.id.clone(),
                            });

                            let sold = to_sell;

                            let buy_price = Decimal::ZERO;
                            let buy_fee = Decimal::ZERO;

                            let (sell_price, _) = rate_provider.convert(&s.price, &stock.date)?;
                            let (sell_fee, _) = rate_provider.convert(&s.fee, &stock.date)?;
                            year_statistics.tax +=
                                buy_price * sold - sell_price * sell_price - buy_fee - sell_fee;

                            to_sell -= sold;
                            debug_assert!(to_sell >= Decimal::ZERO);
                        }
                    };

                    if pop_buy {
                        current_buys.pop_front();
                    }
                }
            }
        }
    }

    let calculated_stock = CalculatedStock {
        ticker,
        provider,
        stats: statistics_by_year.remove(&year).unwrap_or_default(),
        history_by_year: statistics_by_year,
    };

    Ok(calculated_stock)
}
