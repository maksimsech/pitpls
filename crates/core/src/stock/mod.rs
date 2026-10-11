use std::collections::{BTreeMap, BTreeSet};

use chrono::Datelike;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    DecimalExt,
    common::{Country, IsinCountryCode, Mic},
    rate::{NbpRateProvider, RateConverterError},
    tax::POLAND_TAX,
    time::tax_date,
};

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

    // For now a sale counts in its trade year, even when it settles in the next one; the
    // settlement date only picks the NBP rate (docs/stocks/IMPORTANT-QUESTIONS.md, Q1).
    let sales = sales
        .into_iter()
        .filter(|sale| tax_date(sale.time).year() == year)
        .collect::<Vec<_>>();

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
    let countries = country_totals(&sales, income - costs);
    let tax_paid_abroad = countries
        .iter()
        .map(|country| country.used)
        .sum::<Decimal>()
        .round_groszy();
    let warnings = warnings(&sales);

    Ok(StockTaxData {
        income,
        costs,
        tax_paid_abroad,
        sales,
        open_lots,
        holdings,
        countries,
        warnings,
    })
}

// TODO: Revisit D-34 (docs/stocks/12-investigations.md). The per-country limit (art. 30b ust. 5a)
// is an assumption: each country gets its share of the Polish tax, capped at the whole tax, but the
// sum over countries isn't capped yet. Tax paid on a sale without an exchange stays out of D-34.
fn country_totals(sales: &[CalculatedSale], total_profit: Decimal) -> Vec<CountryTotal> {
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

    let polish_tax = if total_profit > Decimal::ZERO {
        POLAND_TAX * total_profit.round_zloty()
    } else {
        Decimal::ZERO
    };

    sums.into_values()
        .map(|sum| {
            let limit = if sum.profit > Decimal::ZERO && total_profit > Decimal::ZERO {
                polish_tax * sum.profit.min(total_profit) / total_profit
            } else {
                Decimal::ZERO
            };
            CountryTotal {
                country: sum.country,
                exchanges: sum.exchanges.into_iter().collect(),
                sales: sum.sales,
                profit: sum.profit,
                income: sum.profit.max(Decimal::ZERO).round_groszy(),
                tax_paid: sum.tax_paid.round_groszy(),
                withheld: sum.tax_paid,
                limit,
                used: sum.tax_paid.min(limit),
            }
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
