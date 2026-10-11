use std::collections::{BTreeMap, HashMap, VecDeque};

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use super::{
    CalculateStocksError,
    model::{
        CalculatedSale, Holding, InstrumentExchange, LotMatch, OpenLot, Replay, SplitStep, Stock,
        StockAction, StockBuy, StockSell, StockSplit,
    },
    settlement::{market, settlement_date},
};
use crate::{
    common::{Amount, Currency},
    rate::NbpRateProvider,
};

pub fn replay<'a>(
    records: impl IntoIterator<Item = &'a Stock>,
    exchanges: &[InstrumentExchange],
    rates: &NbpRateProvider,
) -> Result<Replay, CalculateStocksError> {
    let exchanges: HashMap<_, _> = exchanges
        .iter()
        .map(|exchange| {
            (
                (exchange.ticker.as_str(), exchange.provider.as_str()),
                exchange,
            )
        })
        .collect();

    let mut by_ticker: BTreeMap<&str, Vec<&Stock>> = BTreeMap::new();
    for record in records {
        by_ticker.entry(&record.ticker).or_default().push(record);
    }

    let mut sales = Vec::new();
    let mut open_lots = Vec::new();
    let mut holdings = Vec::new();

    for (ticker, mut records) in by_ticker {
        // TODO: Warn when records of a ticker share a timestamp. Until then they go split, buy,
        // sell, then by ID, so at least the result doesn't change between runs.
        records.sort_by(|a, b| {
            (a.time, order(&a.action), &a.id).cmp(&(b.time, order(&b.action), &b.id))
        });

        // Created up front so a split also gives a holding to providers that buy only after it.
        let mut positions: BTreeMap<&str, Position> = records
            .iter()
            .filter_map(|record| provider(&record.action))
            .map(|provider| {
                let exchange = exchanges.get(&(ticker, provider)).copied();
                (provider, Position::new(ticker, provider, exchange))
            })
            .collect();

        for record in records {
            match &record.action {
                StockAction::Split(split) => {
                    for position in positions.values_mut() {
                        position.split(record, split);
                        holdings.push(position.holding(record));
                    }
                }
                StockAction::Buy(buy) => {
                    let position = positions
                        .get_mut(buy.provider.as_str())
                        .expect("position exists for every provider");
                    position.buy(record, buy, rates)?;
                    holdings.push(position.holding(record));
                }
                StockAction::Sell(sell) => {
                    let position = positions
                        .get_mut(sell.provider.as_str())
                        .expect("position exists for every provider");
                    sales.push(position.sell(record, sell, rates)?);
                    holdings.push(position.holding(record));
                }
            }
        }

        open_lots.extend(positions.into_values().flat_map(Position::into_open_lots));
    }

    sales.sort_by(|a, b| (a.time, &a.id).cmp(&(b.time, &b.id)));

    Ok(Replay {
        sales,
        open_lots,
        holdings,
    })
}

fn order(action: &StockAction) -> u8 {
    match action {
        StockAction::Split(_) => 0,
        StockAction::Buy(_) => 1,
        StockAction::Sell(_) => 2,
    }
}

fn provider(action: &StockAction) -> Option<&str> {
    match action {
        StockAction::Buy(buy) => Some(&buy.provider),
        StockAction::Sell(sell) => Some(&sell.provider),
        StockAction::Split(_) => None,
    }
}

struct Position<'a> {
    ticker: &'a str,
    provider: &'a str,
    exchange: Option<&'a InstrumentExchange>,
    lots: VecDeque<Lot<'a>>,
}

impl<'a> Position<'a> {
    fn new(ticker: &'a str, provider: &'a str, exchange: Option<&'a InstrumentExchange>) -> Self {
        Self {
            ticker,
            provider,
            exchange,
            lots: VecDeque::new(),
        }
    }

    fn settlement(
        &self,
        time: DateTime<Utc>,
        recorded: Option<NaiveDate>,
        currency: Currency,
    ) -> (NaiveDate, bool) {
        match recorded {
            Some(date) => (date, false),
            None => {
                let market = market(self.exchange.map(|exchange| exchange.country), currency);
                (settlement_date(time, market), true)
            }
        }
    }

    fn buy(
        &mut self,
        record: &'a Stock,
        buy: &StockBuy,
        rates: &NbpRateProvider,
    ) -> Result<(), CalculateStocksError> {
        let (settlement, _) = self.settlement(record.time, buy.settlement_date, buy.price.currency);
        let cost = rates
            .convert(&value(buy.number, &buy.price), &settlement)
            .map_err(CalculateStocksError::PriceConversion)?;
        let fee = rates
            .convert(&buy.fee, &settlement)
            .map_err(CalculateStocksError::FeeConversion)?;

        self.lots.push_back(Lot {
            buy_id: &record.id,
            bought: record.time,
            price: buy.price,
            shares: buy.number,
            open: buy.number,
            nbp_date: cost.date,
            nbp_rate: cost.rate,
            cost: cost.pln,
            fee: fee.pln,
            splits: Vec::new(),
        });
        Ok(())
    }

    fn split(&mut self, record: &Stock, split: &StockSplit) {
        let (before, after) = (Decimal::from(split.before), Decimal::from(split.after));
        for lot in &mut self.lots {
            let shares = lot.shares * after / before;
            lot.splits.push(SplitStep {
                split_id: record.id.clone(),
                time: record.time,
                before: split.before,
                after: split.after,
                lot_shares_before: lot.shares,
                lot_shares_after: shares,
            });
            lot.shares = shares;
            lot.open = lot.open * after / before;
        }
    }

    fn sell(
        &mut self,
        record: &Stock,
        sell: &StockSell,
        rates: &NbpRateProvider,
    ) -> Result<CalculatedSale, CalculateStocksError> {
        let (settlement_date, settlement_worked_out) =
            self.settlement(record.time, sell.settlement_date, sell.price.currency);
        let income = rates
            .convert(&value(sell.number, &sell.price), &settlement_date)
            .map_err(CalculateStocksError::PriceConversion)?;
        let fee = rates
            .convert(&sell.fee, &settlement_date)
            .map_err(CalculateStocksError::FeeConversion)?;
        let tax_paid_pln = match &sell.tax_paid {
            Some(tax_paid) => {
                rates
                    .convert(tax_paid, &settlement_date)
                    .map_err(CalculateStocksError::TaxPaidConversion)?
                    .pln
            }
            None => Decimal::ZERO,
        };

        let mut left = sell.number;
        let mut matches = Vec::new();
        while left > Decimal::ZERO
            && let Some(lot) = self.lots.front_mut()
        {
            let taken = lot.open.min(left);
            matches.push(lot.take(taken));
            left -= taken;
            if lot.open.is_zero() {
                self.lots.pop_front();
            }
        }

        let lots_cost = matches.iter().map(|lot| lot.cost).sum::<Decimal>();
        let buy_fees = matches.iter().map(|lot| lot.fee_part).sum::<Decimal>();
        let costs = lots_cost + buy_fees + fee.pln;

        Ok(CalculatedSale {
            id: record.id.clone(),
            ticker: self.ticker.to_owned(),
            provider: self.provider.to_owned(),
            time: record.time,
            settlement_date,
            settlement_worked_out,
            shares: sell.number,
            price: sell.price,
            nbp_date: income.date,
            nbp_rate: income.rate,
            income: income.pln,
            fee: sell.fee,
            fee_pln: fee.pln,
            lots_cost,
            buy_fees,
            costs,
            profit: income.pln - costs,
            tax_paid: sell.tax_paid,
            tax_paid_pln,
            matches,
            uncovered: left.max(Decimal::ZERO),
            exchange: self.exchange.map(|exchange| exchange.mic),
            country: self.exchange.map(|exchange| exchange.country),
        })
    }

    fn holding(&self, record: &Stock) -> Holding {
        Holding {
            record_id: record.id.clone(),
            provider: self.provider.to_owned(),
            shares: self.lots.iter().map(|lot| lot.open).sum(),
            lots: self.lots.len(),
        }
    }

    fn into_open_lots(self) -> impl Iterator<Item = OpenLot> {
        self.lots.into_iter().map(move |lot| OpenLot {
            buy_id: lot.buy_id.to_owned(),
            ticker: self.ticker.to_owned(),
            provider: self.provider.to_owned(),
            bought: lot.bought,
            lot_shares: lot.shares,
            open: lot.open,
            price: lot.price_after_splits(),
            nbp_date: lot.nbp_date,
            nbp_rate: lot.nbp_rate,
            cost: lot.cost * lot.open / lot.shares,
            fee_part: lot.fee * lot.open / lot.shares,
            splits: lot.splits,
        })
    }
}

// Cost and fee stay totals for the whole lot; a split only changes the share count.
struct Lot<'a> {
    buy_id: &'a str,
    bought: DateTime<Utc>,
    price: Amount,
    shares: Decimal,
    open: Decimal,
    nbp_date: NaiveDate,
    nbp_rate: Decimal,
    cost: Decimal,
    fee: Decimal,
    splits: Vec<SplitStep>,
}

impl Lot<'_> {
    fn take(&mut self, taken: Decimal) -> LotMatch {
        let before = self.open;
        self.open -= taken;
        LotMatch {
            buy_id: self.buy_id.to_owned(),
            bought: self.bought,
            lot_shares: self.shares,
            before,
            taken,
            after: self.open,
            price: self.price_after_splits(),
            nbp_date: self.nbp_date,
            nbp_rate: self.nbp_rate,
            cost: self.cost * taken / self.shares,
            fee_part: self.fee * taken / self.shares,
            splits: self.splits.clone(),
        }
    }

    fn price_after_splits(&self) -> Amount {
        let value = self.splits.iter().fold(self.price.value, |price, split| {
            price * Decimal::from(split.before) / Decimal::from(split.after)
        });
        Amount {
            value,
            currency: self.price.currency,
        }
    }
}

fn value(shares: Decimal, price: &Amount) -> Amount {
    Amount {
        value: shares * price.value,
        currency: price.currency,
    }
}
