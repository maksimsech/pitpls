use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

use crate::common::{Amount, Country, Mic};

#[derive(Clone)]
pub struct Stock {
    pub id: String,
    pub time: DateTime<Utc>,
    pub ticker: String,
    pub action: StockAction,
}

impl Stock {
    pub fn provider(&self) -> Option<&str> {
        match &self.action {
            StockAction::Buy(buy) => Some(&buy.provider),
            StockAction::Sell(sell) => Some(&sell.provider),
            StockAction::Split(_) => None,
        }
    }
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

pub struct InstrumentExchange {
    pub ticker: String,
    pub provider: String,
    pub mic: Mic,
    pub country: Country,
}

pub struct Replay {
    pub sales: Vec<CalculatedSale>,
    pub open_lots: Vec<OpenLot>,
    pub holdings: Vec<Holding>,
}

// The state right after the draft record, with the records that follow it left out.
pub enum StockPreview {
    Buy {
        lot: OpenLot,
        holding: Holding,
    },
    Sell {
        sale: CalculatedSale,
        holding: Holding,
    },
    Split {
        lots: Vec<OpenLot>,
        holdings: Vec<Holding>,
    },
}

pub struct StockTaxData {
    pub income: Decimal,
    pub costs: Decimal,
    pub tax_paid_abroad: Decimal,
    pub sales: Vec<CalculatedSale>,
    pub open_lots: Vec<OpenLot>,
    pub countries: Vec<CountryTotal>,
    pub warnings: Vec<Warning>,
}

pub struct CalculatedSale {
    pub id: String,
    pub ticker: String,
    pub provider: String,
    pub time: DateTime<Utc>,
    pub settlement_date: NaiveDate,
    pub settlement_worked_out: bool,
    pub shares: Decimal,
    pub price: Amount,
    pub nbp_date: NaiveDate,
    pub nbp_rate: Decimal,
    pub income: Decimal,
    pub fee: Amount,
    pub fee_pln: Decimal,
    pub lots_cost: Decimal,
    pub buy_fees: Decimal,
    pub costs: Decimal,
    pub profit: Decimal,
    pub tax_paid: Option<Amount>,
    pub tax_paid_pln: Decimal,
    pub matches: Vec<LotMatch>,
    pub uncovered: Decimal,
    pub exchange: Option<Mic>,
    pub country: Option<Country>,
}

pub struct LotMatch {
    pub buy_id: String,
    pub bought: DateTime<Utc>,
    pub lot_shares: Decimal,
    pub before: Decimal,
    pub taken: Decimal,
    pub after: Decimal,
    pub price: Amount,
    pub nbp_date: NaiveDate,
    pub nbp_rate: Decimal,
    pub cost: Decimal,
    pub fee_part: Decimal,
    pub splits: Vec<SplitStep>,
}

pub struct OpenLot {
    pub buy_id: String,
    pub ticker: String,
    pub provider: String,
    pub bought: DateTime<Utc>,
    pub lot_shares: Decimal,
    pub open: Decimal,
    pub price: Amount,
    pub nbp_date: NaiveDate,
    pub nbp_rate: Decimal,
    pub cost: Decimal,
    pub fee_part: Decimal,
    pub splits: Vec<SplitStep>,
}

#[derive(Clone)]
pub struct SplitStep {
    pub split_id: String,
    pub time: DateTime<Utc>,
    pub before: u32,
    pub after: u32,
    pub lot_shares_before: Decimal,
    pub lot_shares_after: Decimal,
}

// Shares held by one provider after a record; a split gets one per provider of its ticker.
pub struct Holding {
    pub record_id: String,
    pub provider: String,
    pub shares: Decimal,
    pub lots: usize,
}

#[derive(Debug)]
pub struct CountryTotal {
    pub country: Country,
    pub exchanges: Vec<Mic>,
    pub sales: usize,
    pub profit: Decimal,
    // PIT/ZG field 29, 0 for a loss.
    pub income: Decimal,
    // PIT/ZG field 30, without the D-34 limit.
    pub tax_paid: Decimal,
    pub limit: Decimal,
    pub used: Decimal,
}

pub enum Warning {
    SellWithoutBuy {
        sell_id: String,
        shares: Decimal,
    },
    MissingExchange {
        ticker: String,
        provider: String,
        sales: usize,
    },
}
