use std::collections::HashMap;

use anyhow::Result;
use chrono::Datelike;

use crate::rate::NbpRateProvider;

mod model;

pub use model::{Stock, StockAction, StockBuy, StockSell, StockSplit};

/*
 * How algorithm should work:
 * 1. If there is a sell in this year -> I need to calculate cost basis.
 * 2. To calculate cost basis I need full historical data for current + past years.
 * 3. I should remember that I have to use FIFO, so I need to calculate historical information always from start.
 * 4. Key for calculation should be ticker + provider.
 * 4. As an optimisation for step two I could remove from calculation fully closed position.
 */

/*
 * For now just accept full stock data, later I could optimize it.
 */
pub fn calculate_sell_buy_values(
    stocks: Vec<Stock>,
    year: i32,
    rate_provider: &NbpRateProvider,
) -> Result<()> {
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

    for stock in stocks {}
    Ok(())
}

struct TickerResult {}
