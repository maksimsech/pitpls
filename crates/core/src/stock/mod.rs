use std::collections::{BTreeMap, BTreeSet};

use chrono::Datelike;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    DecimalExt,
    common::{Country, IsinCountryCode, Mic},
    rate::{NbpRateProvider, RateConverterError},
    time::tax_date,
};

use settlement::settles_on;

mod model;
mod preview;
mod replay;
mod settlement;
#[cfg(test)]
mod tests;

pub use model::{
    CalculatedSale, CountryTotal, Holding, Instrument, InstrumentExchange, LotMatch, OpenLot,
    Replay, SplitStep, Stock, StockAction, StockBuy, StockPreview, StockSell, StockSplit,
    StockTaxData, Warning,
};
pub use preview::preview;
pub use replay::{replay, sort_key};
pub use settlement::{Market, market, settlement_date};

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum CalculateStocksError {
    PriceConversion(#[source] RateConverterError),
    FeeConversion(#[source] RateConverterError),
    TaxPaidConversion(#[source] RateConverterError),
}

pub fn calculate_stocks(
    records: &[Stock],
    exchanges: &[InstrumentExchange],
    year: i32,
    rates: &NbpRateProvider,
) -> Result<StockTaxData, CalculateStocksError> {
    // Later records don't count in this year, and open lots are wanted on 31.12.
    let records = records
        .iter()
        .filter(|record| tax_date(record.time).year() <= year);
    let Replay {
        sales,
        open_lots,
        holdings,
    } = replay(records, exchanges, rates)?;

    // A sale counts in the year it settles, like on podatekgieldowy.pl: one traded on 30.12 that
    // settles on 02.01 is next year's tax (docs/stocks/IMPORTANT-QUESTIONS.md, Q1).
    let (sales, next_year_sales): (Vec<_>, Vec<_>) = sales
        .into_iter()
        .filter(|sale| sale.settlement_date.year() >= year)
        .partition(|sale| sale.settlement_date.year() == year);

    let income = sales
        .iter()
        .map(|sale| sale.income)
        .sum::<Decimal>()
        .round_groszy();
    let costs = sales
        .iter()
        .map(|sale| sale.costs)
        .sum::<Decimal>()
        .round_groszy();
    let countries = country_totals(&sales);
    let warnings = warnings(&sales);

    Ok(StockTaxData {
        income,
        costs,
        sales,
        next_year_sales,
        open_lots,
        holdings,
        countries,
        warnings,
    })
}

// No D-34 for now: its limit needs the whole taxable income, which the app doesn't know
// (docs/stocks/d34-tax-paid-abroad.md). Tax paid on a sale without an exchange stays out of PIT/ZG.
fn country_totals(sales: &[CalculatedSale]) -> Vec<CountryTotal> {
    struct Sum {
        country: Country,
        exchanges: BTreeSet<Mic>,
        sales: usize,
        profit: Decimal,
        tax_paid: Decimal,
    }

    let mut sums: BTreeMap<IsinCountryCode, Sum> = BTreeMap::new();
    for sale in sales {
        let (Some(country), Some(exchange)) = (sale.country, sale.exchange) else {
            continue;
        };
        let sum = sums.entry(country.code()).or_insert_with(|| Sum {
            country,
            exchanges: BTreeSet::new(),
            sales: 0,
            profit: Decimal::ZERO,
            tax_paid: Decimal::ZERO,
        });
        sum.exchanges.insert(exchange);
        sum.sales += 1;
        sum.profit += sale.profit;
        sum.tax_paid += sale.tax_paid_pln;
    }

    sums.into_values()
        .map(|sum| CountryTotal {
            country: sum.country,
            exchanges: sum.exchanges.into_iter().collect(),
            sales: sum.sales,
            profit: sum.profit,
            income: sum.profit.max(Decimal::ZERO).round_groszy(),
            tax_paid: sum.tax_paid.round_groszy(),
        })
        .collect()
}

// The years sales count in, worked out without rates. Records lists a sale under its trade year,
// so a sale traded on 30.12 can be all its tax year has.
pub fn sale_years(records: &[Stock], exchanges: &[InstrumentExchange]) -> BTreeSet<i32> {
    records
        .iter()
        .filter_map(|record| {
            let StockAction::Sell(sell) = &record.action else {
                return None;
            };
            let country = exchanges
                .iter()
                .find(|exchange| {
                    exchange.ticker == record.ticker && exchange.provider == sell.provider
                })
                .map(|exchange| exchange.country);
            let (date, _) = settles_on(
                record.time,
                sell.settlement_date,
                country,
                sell.price.currency,
            );
            Some(date.year())
        })
        .collect()
}

fn warnings(sales: &[CalculatedSale]) -> Vec<Warning> {
    let mut warnings = sales
        .iter()
        .filter(|sale| sale.uncovered > Decimal::ZERO)
        .map(|sale| Warning::SellWithoutBuy {
            sell_id: sale.id.clone(),
            shares: sale.uncovered,
        })
        .collect::<Vec<_>>();

    let mut missing: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for sale in sales.iter().filter(|sale| sale.exchange.is_none()) {
        *missing.entry((&sale.ticker, &sale.provider)).or_default() += 1;
    }
    warnings.extend(missing.into_iter().map(|((ticker, provider), sales)| {
        Warning::MissingExchange {
            ticker: ticker.to_owned(),
            provider: provider.to_owned(),
            sales,
        }
    }));

    warnings
}
