use super::{counted, set_exchange::ExchangeRow};
use crate::{
    components::records::{EntryStyle, LotRow, LotTable, RowDisplay, Step, StepLine},
    format::{DisplayText, amount, date, money, pln, shares},
    pages::records::{date_label, day},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use gpui_kit::{SharedString, assets::IconName};
use pitpls_app::use_case::stock::{Listing, Listings, StockData};
use pitpls_core::{
    common::{Amount, Currency},
    stock::{
        CalculatedSale, Holding, LotMatch, SplitStep, Stock, StockAction, StockBuy, StockSell,
        StockSplit, StockTaxData, Warning,
    },
    time,
};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Buy,
    Sell,
    Split,
}

impl Action {
    pub fn title(self) -> &'static str {
        match self {
            Self::Buy => "Buy",
            Self::Sell => "Sell",
            Self::Split => "Split",
        }
    }
}

pub struct WarningCard {
    pub icon: IconName,
    pub title: SharedString,
    pub description: SharedString,
    pub action: WarningAction,
}

#[derive(Clone)]
pub enum WarningAction {
    AddBuy(Option<BuyPrefill>),
    SetExchange(Vec<ExchangeRow>),
}

#[derive(Clone)]
pub struct BuyPrefill {
    pub ticker: String,
    pub provider: String,
    pub currency: Currency,
    pub first_sale: DateTime<Utc>,
}

pub struct HoldingBlock {
    pub ticker: SharedString,
    pub provider: SharedString,
    pub exchange: Option<SharedString>,
    pub shares: Decimal,
    pub lots: Vec<RowDisplay>,
    pub search: String,
}

pub struct SoldOut {
    pub instrument: SharedString,
    pub last_sale: SharedString,
    pub search: String,
}

#[derive(Clone)]
pub struct InstrumentKey {
    pub ticker: SharedString,
    pub provider: SharedString,
}

impl InstrumentKey {
    fn new(ticker: &str, provider: &str) -> Self {
        Self {
            ticker: ticker.to_owned().into(),
            provider: provider.to_owned().into(),
        }
    }
}

pub struct StockDisplay {
    pub stats: Vec<(&'static str, DisplayText)>,
    pub warnings: Vec<WarningCard>,
    pub sales: Vec<RowDisplay>,
    pub profits: Vec<Decimal>,
    pub records: Vec<RowDisplay>,
    pub stocks: Vec<Stock>,
    pub actions: Vec<Action>,
    pub holdings: Vec<HoldingBlock>,
    pub sold_out: Vec<SoldOut>,
    /// In the order of the row's history links.
    pub histories: HashMap<SharedString, Vec<InstrumentKey>>,
}

impl StockDisplay {
    pub fn new(data: &StockData, year: i32) -> Self {
        let tax = &data.tax;
        let listings = &data.listings;
        let mut histories = HashMap::new();
        let sales = tax
            .sales
            .iter()
            .map(|sale| {
                let row = sale_row(sale, holding_after(tax, sale), listings);
                histories.insert(
                    row.id.clone(),
                    vec![InstrumentKey::new(&sale.ticker, &sale.provider)],
                );
                row
            })
            .collect();
        let records = data
            .records
            .iter()
            .map(|record| {
                let keys = history_keys(record, tax);
                let row = record_row(record, &keys, tax, listings, year);
                histories.insert(row.id.clone(), keys);
                row
            })
            .collect();
        let (holdings, sold_out) = holdings(tax, listings);
        Self {
            stats: vec![
                ("Income (C-22)", pln(tax.income)),
                ("Costs (C-23)", pln(tax.costs)),
            ],
            warnings: warnings(tax, year),
            sales,
            profits: tax.sales.iter().map(|sale| sale.profit).collect(),
            records,
            stocks: data.records.clone(),
            actions: data.records.iter().map(action).collect(),
            holdings,
            sold_out,
            histories,
        }
    }
}

fn holding_after<'a>(tax: &'a StockTaxData, sale: &CalculatedSale) -> Option<&'a Holding> {
    tax.holdings
        .iter()
        .find(|holding| holding.record_id == sale.id && holding.provider == sale.provider)
}

/// A split has a history in every provider that held or traded the ticker by
/// then.
fn history_keys(record: &Stock, tax: &StockTaxData) -> Vec<InstrumentKey> {
    match record.provider() {
        Some(provider) => vec![InstrumentKey::new(&record.ticker, provider)],
        None => tax
            .holdings
            .iter()
            .filter(|holding| holding.record_id == record.id)
            .map(|holding| InstrumentKey::new(&record.ticker, &holding.provider))
            .collect(),
    }
}

fn history_links(ticker: &str, keys: &[InstrumentKey]) -> Vec<SharedString> {
    match keys {
        [_] => vec![format!("{ticker} history").into()],
        keys => keys
            .iter()
            .map(|key| format!("{ticker} history · {}", key.provider).into())
            .collect(),
    }
}

fn action(record: &Stock) -> Action {
    match record.action {
        StockAction::Buy(_) => Action::Buy,
        StockAction::Sell(_) => Action::Sell,
        StockAction::Split(_) => Action::Split,
    }
}

pub fn shares_text(value: Decimal) -> String {
    shares(value).full.to_string()
}

pub fn shares_count(value: Decimal) -> String {
    let unit = if value == Decimal::ONE {
        "share"
    } else {
        "shares"
    };
    format!("{} {unit}", shares_text(value))
}

pub fn lots_count(lots: usize) -> String {
    match lots {
        1 => "1 lot".to_owned(),
        lots => format!("{lots} lots"),
    }
}

fn day_month(value: NaiveDate) -> String {
    value.format("%d.%m").to_string()
}

pub fn trade_date(value: DateTime<Utc>) -> String {
    date(time::tax_date(value)).main.to_string()
}

pub fn price_text(value: Amount) -> String {
    amount(value).full.to_string()
}

pub fn ratio(before: u32, after: u32) -> String {
    format!("{after}-for-{before}")
}

pub fn rate_cell(rate: Decimal, nbp_date: NaiveDate, currency: Currency) -> DisplayText {
    if currency == Currency::PLN {
        return DisplayText::plain("—");
    }
    DisplayText {
        extra: day_month(nbp_date).into(),
        ..DisplayText::plain(rate.to_string())
    }
}

pub fn conversion(shares_value: Decimal, price: Amount, rate: Decimal) -> Vec<DisplayText> {
    let mut parts = vec![shares(shares_value), DisplayText::plain("×"), amount(price)];
    if price.currency != Currency::PLN {
        parts.push(DisplayText::plain(format!("× {rate}")));
    }
    parts
}

/// A fee in another currency than the price has its own rate, which the
/// calculation keeps only inside its PLN value.
pub fn fee_rate(fee: Amount, fee_pln: Decimal, price: Amount, rate: Decimal) -> Decimal {
    if fee.currency == price.currency || fee.value.is_zero() {
        rate
    } else {
        fee_pln / fee.value
    }
}

fn exchange_name(listing: Option<&Listing>) -> Option<SharedString> {
    listing.map(|listing| listing.name.clone().into())
}

fn exchange_cell(name: Option<&SharedString>) -> DisplayText {
    name.map_or_else(
        || DisplayText::plain("Missing"),
        |name| DisplayText::plain(name.clone()),
    )
}

fn entry(label: impl Into<SharedString>, value: Vec<DisplayText>, style: EntryStyle) -> StepLine {
    StepLine::Entry {
        label: label.into(),
        value,
        style,
    }
}

fn added(value: Decimal) -> DisplayText {
    let value = pln(value);
    DisplayText {
        main: format!("+{}", value.main).into(),
        ..value
    }
}

fn sale_row(sale: &CalculatedSale, holding: Option<&Holding>, listings: &Listings) -> RowDisplay {
    let listing = listings
        .get(&sale.ticker, &sale.provider)
        .filter(|_| sale.exchange.is_some());
    let exchange = exchange_name(listing);
    let cells = vec![
        day(sale.time),
        DisplayText::plain(sale.ticker.clone()),
        exchange_cell(exchange.as_ref()),
        DisplayText::plain(sale.provider.clone()),
        shares(sale.shares),
        pln(sale.income),
        pln(sale.costs),
        pln(sale.profit),
    ];
    let link = format!("Full {} history", sale.ticker);
    RowDisplay {
        id: sale.id.clone().into(),
        time: sale.time,
        cells,
        steps: sale_steps(sale, listing),
        lots: Some(fifo_table(sale, holding, Some(link.into()))),
        flagged: if exchange.is_none() { vec![2] } else { vec![] },
        search: search(&sale.ticker, &sale.provider, exchange.as_ref(), "sell"),
        label: format!("{} sale {}", sale.ticker, date_label(sale.time)).into(),
        links: vec![],
    }
}

fn search(ticker: &str, provider: &str, exchange: Option<&SharedString>, action: &str) -> String {
    let exchange = exchange.map_or("", |exchange| exchange.as_ref());
    format!("{ticker}\n{provider}\n{exchange}\n{action}").to_lowercase()
}

fn sale_steps(sale: &CalculatedSale, listing: Option<&Listing>) -> Vec<Step> {
    let settled = if sale.settlement_worked_out {
        format!(
            "Settled {}, worked out from the trade date",
            date(sale.settlement_date).main
        )
    } else {
        format!("Settled {}", date(sale.settlement_date).main)
    };
    let nbp = if sale.price.currency == Currency::PLN {
        "PLN, no conversion".to_owned()
    } else {
        format!("NBP date {}", date(sale.nbp_date).main)
    };
    let counts_in = counts_in(sale)
        .map(|year| format!(" for {year}"))
        .unwrap_or_default();

    let mut costs = vec![
        entry(
            "Lots taken (FIFO)",
            vec![pln(sale.lots_cost)],
            EntryStyle::Normal,
        ),
        entry(
            "Buy fees, part taken",
            vec![pln(sale.buy_fees)],
            EntryStyle::Normal,
        ),
    ];
    if sale.uncovered > Decimal::ZERO {
        costs.push(entry(
            format!("{} with no buy", shares_count(sale.uncovered)),
            vec![pln(Decimal::ZERO)],
            EntryStyle::Warning,
        ));
    }
    costs.extend([
        entry(
            fee_label(
                "Sale fee",
                sale.fee,
                sale.fee_pln,
                sale.price,
                sale.nbp_rate,
            ),
            vec![pln(sale.fee_pln)],
            EntryStyle::Normal,
        ),
        entry("Costs", vec![pln(sale.costs)], EntryStyle::Strong),
        StepLine::Caption(format!("Adds to Costs (C-23){counts_in}").into()),
    ]);

    let result = if sale.profit < Decimal::ZERO {
        "Loss"
    } else {
        "Profit"
    };
    let tax_paid = sale
        .tax_paid
        .map(|_| format!(" · tax paid abroad {} PLN", money(sale.tax_paid_pln).full));
    let pit_zg = match (sale.country, listing) {
        (Some(country), Some(listing)) => format!(
            "PIT/ZG · {country} ({}) · tax paid abroad {} PLN",
            listing.name,
            money(sale.tax_paid_pln).full
        ),
        _ => format!(
            "PIT/ZG · no country until the exchange is set{}",
            tax_paid.unwrap_or_default()
        ),
    };

    vec![
        Step {
            title: "Income",
            lines: vec![
                StepLine::Formula(conversion(sale.shares, sale.price, sale.nbp_rate)),
                StepLine::Result(vec![pln(sale.income)]),
                StepLine::Caption(
                    format!("{settled} · {nbp} · adds to Income (C-22){counts_in}").into(),
                ),
            ],
        },
        Step {
            title: "Costs",
            lines: costs,
        },
        Step {
            title: "Result",
            lines: vec![
                StepLine::Formula(vec![
                    money(sale.income),
                    DisplayText::plain("−"),
                    money(sale.costs),
                ]),
                StepLine::Result(vec![DisplayText::plain(result), pln(sale.profit.abs())]),
                StepLine::Caption(pit_zg.into()),
            ],
        },
    ]
}

// The year's sales and the late-December ones that settle, and count, in the next year.
fn all_sales(tax: &StockTaxData) -> impl Iterator<Item = &CalculatedSale> {
    tax.sales.iter().chain(&tax.next_year_sales)
}

// A sale counts in the year it settles, so one traded in late December can count in the next.
fn counts_in(sale: &CalculatedSale) -> Option<i32> {
    let year = sale.settlement_date.year();
    (year != time::tax_date(sale.time).year()).then_some(year)
}

fn fee_label(name: &str, fee: Amount, fee_pln: Decimal, price: Amount, rate: Decimal) -> String {
    if fee.value.is_zero() {
        name.to_owned()
    } else if fee.currency == Currency::PLN {
        format!("{name} in PLN, no conversion")
    } else {
        format!(
            "{name} {} × {}",
            price_text(fee),
            fee_rate(fee, fee_pln, price, rate)
        )
    }
}

/// The holding before the sale is the one after it plus what it took; the
/// shares with no buy were never held.
fn fifo_table(
    sale: &CalculatedSale,
    holding: Option<&Holding>,
    link: Option<SharedString>,
) -> LotTable {
    let (after, after_lots) =
        holding.map_or((Decimal::ZERO, 0), |holding| (holding.shares, holding.lots));
    let taken = sale.matches.iter().map(|lot| lot.taken).sum::<Decimal>();
    let closed = sale
        .matches
        .iter()
        .filter(|lot| lot.after.is_zero())
        .count();
    let before = match after_lots + closed {
        0 => "No shares".to_owned(),
        lots => format!("{} in {}", shares_count(after + taken), lots_count(lots)),
    };
    let caption = if sale.uncovered > Decimal::ZERO {
        format!(
            "{before} before this sale, {} sold",
            shares_count(sale.shares)
        )
    } else if after_lots == 0 {
        format!("{before} before this sale → 0 after")
    } else {
        format!(
            "{before} before this sale → {} in {} after",
            shares_text(after),
            lots_count(after_lots)
        )
    };

    let dash = || DisplayText::plain("—");
    let mut rows = sale.matches.iter().map(lot_match_row).collect::<Vec<_>>();
    if sale.uncovered > Decimal::ZERO {
        rows.push(LotRow {
            cells: vec![
                DisplayText::plain("No buy"),
                dash(),
                dash(),
                shares(sale.uncovered),
                dash(),
                dash(),
                dash(),
                money(Decimal::ZERO),
                money(Decimal::ZERO),
            ],
            uncovered: true,
            notes: vec![],
        });
    }
    LotTable {
        title: format!("FIFO · {} in {}", sale.ticker, sale.provider).into(),
        caption: caption.into(),
        rows,
        link,
    }
}

fn lot_match_row(lot: &LotMatch) -> LotRow {
    LotRow {
        cells: vec![
            DisplayText::plain(trade_date(lot.bought)),
            shares(lot.lot_shares),
            shares(lot.before),
            shares(lot.taken),
            shares(lot.after),
            amount(lot.price),
            rate_cell(lot.nbp_rate, lot.nbp_date, lot.price.currency),
            money(lot.cost),
            money(lot.fee_part),
        ],
        uncovered: false,
        notes: lot.splits.iter().map(split_note).collect(),
    }
}

fn split_note(split: &SplitStep) -> SharedString {
    format!(
        "Split {} on {}: {} at {} → {} at {}",
        ratio(split.before, split.after),
        trade_date(split.time),
        shares_text(split.lot_shares_before),
        money(split.price_before.value).full,
        shares_text(split.lot_shares_after),
        price_text(split.price_after),
    )
    .into()
}

fn record_row(
    record: &Stock,
    histories: &[InstrumentKey],
    tax: &StockTaxData,
    listings: &Listings,
    year: i32,
) -> RowDisplay {
    let action = action(record);
    let dash = || DisplayText::plain("—");
    let (cells, exchange, steps, lots) = match &record.action {
        StockAction::Buy(StockBuy {
            number,
            price,
            fee,
            provider,
            ..
        })
        | StockAction::Sell(StockSell {
            number,
            price,
            fee,
            provider,
            ..
        }) => {
            let exchange = exchange_name(listings.get(&record.ticker, provider));
            let cells = vec![
                day(record.time),
                DisplayText::plain(action.title()),
                DisplayText::plain(record.ticker.clone()),
                exchange_cell(exchange.as_ref()),
                DisplayText::plain(provider.clone()),
                shares(*number),
                amount(*price),
                amount(Amount {
                    value: number * price.value,
                    currency: price.currency,
                }),
                amount(*fee),
            ];
            let (steps, lots) = match &record.action {
                StockAction::Buy(buy) => (buy_steps(record, buy, tax, year), None),
                _ => match all_sales(tax).find(|sale| sale.id == record.id) {
                    Some(sale) => {
                        let listing = listings
                            .get(&sale.ticker, &sale.provider)
                            .filter(|_| sale.exchange.is_some());
                        (
                            sale_steps(sale, listing),
                            Some(fifo_table(sale, holding_after(tax, sale), None)),
                        )
                    }
                    None => (vec![], None),
                },
            };
            (cells, exchange, steps, lots)
        }
        StockAction::Split(split) => {
            let cells = vec![
                day(record.time),
                DisplayText::plain(action.title()),
                DisplayText::plain(record.ticker.clone()),
                dash(),
                DisplayText::plain("All providers"),
                DisplayText::plain(format!("{} → {}", split.before, split.after)),
                dash(),
                dash(),
                dash(),
            ];
            (cells, None, split_steps(record, split, tax), None)
        }
    };
    let flagged = if exchange.is_none() && action != Action::Split {
        vec![3]
    } else {
        vec![]
    };
    let provider = record.provider().unwrap_or("");
    RowDisplay {
        id: record.id.clone().into(),
        time: record.time,
        cells,
        steps,
        lots,
        flagged,
        search: search(
            &record.ticker,
            provider,
            exchange.as_ref(),
            &action.title().to_lowercase(),
        ),
        label: format!(
            "{} {} {}",
            action.title(),
            record.ticker,
            date_label(record.time)
        )
        .into(),
        links: history_links(&record.ticker, histories),
    }
}

struct LotView<'a> {
    nbp_rate: Decimal,
    nbp_date: NaiveDate,
    cost: Decimal,
    fee: Decimal,
    splits: &'a [SplitStep],
}

/// A buy's lot shows up in the sales that took from it and, if any of it is
/// left, among the open lots; all of them fall in the buy's year.
fn buy_steps(record: &Stock, buy: &StockBuy, tax: &StockTaxData, year: i32) -> Vec<Step> {
    let taken = all_sales(tax)
        .flat_map(|sale| {
            sale.matches
                .iter()
                .filter(|lot| lot.buy_id == record.id)
                .map(move |lot| (sale, lot))
        })
        .collect::<Vec<_>>();
    let open = tax.open_lots.iter().find(|lot| lot.buy_id == record.id);
    let lot = match (open, taken.last()) {
        (Some(open), _) => Some(LotView {
            nbp_rate: open.nbp_rate,
            nbp_date: open.nbp_date,
            cost: open.lot_cost,
            fee: open.lot_fee,
            splits: &open.splits,
        }),
        (None, Some((_, lot))) => Some(LotView {
            nbp_rate: lot.nbp_rate,
            nbp_date: lot.nbp_date,
            cost: lot.lot_cost,
            fee: lot.lot_fee,
            splits: &lot.splits,
        }),
        (None, None) => None,
    };
    let Some(lot) = lot else {
        return vec![];
    };

    let nbp = if buy.price.currency == Currency::PLN {
        "PLN, no conversion".to_owned()
    } else {
        format!("NBP date {}", date(lot.nbp_date).main)
    };
    let fee_formula = if buy.fee.value.is_zero() {
        vec![DisplayText::plain("No fee")]
    } else if buy.fee.currency == Currency::PLN {
        vec![amount(buy.fee)]
    } else {
        conversion_fee(buy.fee, fee_rate(buy.fee, lot.fee, buy.price, lot.nbp_rate))
    };
    let fee_caption = if buy.fee.currency == Currency::PLN && !buy.fee.value.is_zero() {
        "PLN, no conversion · counted in costs as the shares are sold"
    } else {
        "Counted in costs as the shares are sold"
    };

    let mut events = vec![(
        record.time,
        entry(
            format!("Bought {}", trade_date(record.time)),
            vec![shares(buy.number)],
            EntryStyle::Normal,
        ),
    )];
    events.extend(lot.splits.iter().map(|split| {
        (
            split.time,
            entry(
                format!(
                    "Split {} {}",
                    ratio(split.before, split.after),
                    trade_date(split.time)
                ),
                vec![
                    shares(split.lot_shares_before),
                    DisplayText::plain("→"),
                    shares(split.lot_shares_after),
                ],
                EntryStyle::Normal,
            ),
        )
    }));
    events.extend(taken.iter().map(|(sale, lot)| {
        (
            sale.time,
            entry(
                match counts_in(sale) {
                    Some(year) => format!("Sold {} · counts in {year}", trade_date(sale.time)),
                    None => format!("Sold {}", trade_date(sale.time)),
                },
                vec![
                    shares(lot.taken),
                    DisplayText::plain("→ Costs"),
                    added(lot.cost + lot.fee_part),
                ],
                EntryStyle::Normal,
            ),
        )
    }));
    events.sort_by_key(|(time, _)| *time);
    let open_shares = open.map_or(Decimal::ZERO, |lot| lot.open);
    let mut open_value = vec![shares(open_shares)];
    if !open_shares.is_zero() {
        open_value.push(DisplayText::plain(format!("· moves into {}", year + 1)));
    }
    let mut lines = events.into_iter().map(|(_, line)| line).collect::<Vec<_>>();
    lines.push(entry(
        format!("Open on 31.12.{year}"),
        open_value,
        EntryStyle::Strong,
    ));

    vec![
        Step {
            title: "Cost",
            lines: vec![
                StepLine::Formula(conversion(buy.number, buy.price, lot.nbp_rate)),
                StepLine::Result(vec![pln(lot.cost)]),
                StepLine::Caption(format!("{nbp} · a cost only when its shares are sold").into()),
            ],
        },
        Step {
            title: "Fee",
            lines: vec![
                StepLine::Formula(fee_formula),
                StepLine::Result(vec![pln(lot.fee)]),
                StepLine::Caption(fee_caption.into()),
            ],
        },
        Step {
            title: "What happened to the lot",
            lines,
        },
    ]
}

fn conversion_fee(fee: Amount, rate: Decimal) -> Vec<DisplayText> {
    vec![amount(fee), DisplayText::plain(format!("× {rate}"))]
}

/// A lot the split changed is open on 31.12, or taken by a later sale of the
/// year.
fn split_steps(record: &Stock, split: &StockSplit, tax: &StockTaxData) -> Vec<Step> {
    let mut changed: BTreeMap<(&str, DateTime<Utc>, &str), &SplitStep> = BTreeMap::new();
    let taken = all_sales(tax).flat_map(|sale| {
        sale.matches.iter().map(move |lot| {
            (
                sale.provider.as_str(),
                lot.bought,
                lot.buy_id.as_str(),
                &lot.splits,
            )
        })
    });
    let open = tax.open_lots.iter().map(|lot| {
        (
            lot.provider.as_str(),
            lot.bought,
            lot.buy_id.as_str(),
            &lot.splits,
        )
    });
    for (provider, bought, buy_id, splits) in taken.chain(open) {
        if let Some(step) = splits.iter().find(|step| step.split_id == record.id) {
            changed.insert((provider, bought, buy_id), step);
        }
    }
    let mut lots = changed
        .into_iter()
        .map(|((provider, bought, _), step)| {
            entry(
                format!("{provider} · lot {}", trade_date(bought)),
                vec![DisplayText::plain(format!(
                    "{} at {} → {} at {}",
                    shares_text(step.lot_shares_before),
                    money(step.price_before.value).full,
                    shares_text(step.lot_shares_after),
                    price_text(step.price_after),
                ))],
                EntryStyle::Normal,
            )
        })
        .collect::<Vec<_>>();
    if lots.is_empty() {
        lots.push(StepLine::Caption("No open lots on that day".into()));
    } else {
        lots.push(StepLine::Caption(
            "Lot costs in PLN don't change, only share counts and prices".into(),
        ));
    }

    vec![
        Step {
            title: "Split",
            lines: vec![
                StepLine::Formula(vec![DisplayText::plain(format!(
                    "New shares = shares × {} ÷ {}",
                    split.after, split.before
                ))]),
                StepLine::Result(vec![DisplayText::plain(ratio(split.before, split.after))]),
                StepLine::Caption(
                    format!("Changes every {} lot, in every provider", record.ticker).into(),
                ),
            ],
        },
        Step {
            title: "Lots it changed",
            lines: lots,
        },
    ]
}

fn holdings(tax: &StockTaxData, listings: &Listings) -> (Vec<HoldingBlock>, Vec<SoldOut>) {
    let mut blocks: Vec<HoldingBlock> = Vec::new();
    for lot in &tax.open_lots {
        let index = match blocks
            .iter()
            .position(|block| block.ticker == lot.ticker && block.provider == lot.provider)
        {
            Some(index) => index,
            None => {
                let exchange = exchange_name(listings.get(&lot.ticker, &lot.provider));
                blocks.push(HoldingBlock {
                    ticker: lot.ticker.clone().into(),
                    provider: lot.provider.clone().into(),
                    search: search(&lot.ticker, &lot.provider, exchange.as_ref(), ""),
                    exchange,
                    shares: Decimal::ZERO,
                    lots: Vec::new(),
                });
                blocks.len() - 1
            }
        };
        let block = &mut blocks[index];
        block.shares += lot.open;
        block.lots.push(RowDisplay {
            id: lot.buy_id.clone().into(),
            time: lot.bought,
            cells: vec![
                DisplayText {
                    extra: time::local(lot.bought).format("%H:%M").to_string().into(),
                    ..DisplayText::plain(trade_date(lot.bought))
                },
                shares(lot.lot_shares),
                shares(lot.open),
                amount(lot.price),
                rate_cell(lot.nbp_rate, lot.nbp_date, lot.price.currency),
                money(lot.cost),
                money(lot.fee_part),
            ],
            ..Default::default()
        });
    }
    blocks.sort_by(|a, b| (&a.ticker, &a.provider).cmp(&(&b.ticker, &b.provider)));

    let mut last_sales: BTreeMap<(&str, &str), &CalculatedSale> = BTreeMap::new();
    for sale in all_sales(tax) {
        last_sales.insert((&sale.ticker, &sale.provider), sale);
    }
    let sold_out = last_sales
        .into_iter()
        .filter(|((ticker, provider), _)| {
            !blocks
                .iter()
                .any(|block| block.ticker == *ticker && block.provider == *provider)
        })
        .map(|((ticker, provider), sale)| {
            let exchange = exchange_name(listings.get(ticker, provider));
            let instrument = match &exchange {
                Some(exchange) => format!("{ticker} · {exchange} · {provider}"),
                None => format!("{ticker} · {provider}"),
            };
            SoldOut {
                instrument: instrument.into(),
                last_sale: format!("last sale {}", trade_date(sale.time)).into(),
                search: search(ticker, provider, exchange.as_ref(), ""),
            }
        })
        .collect();
    (blocks, sold_out)
}

pub fn names(names: &[&str]) -> String {
    const SHOWN: usize = 4;
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        _ if names.len() > SHOWN => format!(
            "{} and {} more",
            names[..SHOWN - 1].join(", "),
            names.len() - (SHOWN - 1)
        ),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn warnings(tax: &StockTaxData, year: i32) -> Vec<WarningCard> {
    let mut uncovered: Vec<(&str, &str, Currency, Decimal, DateTime<Utc>)> = Vec::new();
    let mut missing: Vec<&str> = Vec::new();
    let mut rows = Vec::new();
    for warning in &tax.warnings {
        match warning {
            Warning::SellWithoutBuy { sell_id, shares } => {
                let Some(sale) = tax.sales.iter().find(|sale| &sale.id == sell_id) else {
                    continue;
                };
                match uncovered.iter_mut().find(|(ticker, provider, ..)| {
                    *ticker == sale.ticker && *provider == sale.provider
                }) {
                    Some((_, _, _, sum, _)) => *sum += shares,
                    None => uncovered.push((
                        &sale.ticker,
                        &sale.provider,
                        sale.price.currency,
                        *shares,
                        sale.time,
                    )),
                }
            }
            Warning::MissingExchange {
                ticker,
                provider,
                sales,
            } => {
                if !missing.contains(&ticker.as_str()) {
                    missing.push(ticker);
                }
                rows.push(missing_row(tax, ticker, provider, *sales, year));
            }
        }
    }

    let mut cards = Vec::new();
    if let [(ticker, provider, currency, shares, first_sale)] = uncovered[..] {
        cards.push(WarningCard {
            icon: IconName::TriangleAlert,
            title: format!("{ticker} sold {} with no buy", shares_count(shares)).into(),
            description: format!(
                "They count with cost 0.00, so Costs (C-23) are too low. Import the earlier \
                 {provider} statement or add the buy."
            )
            .into(),
            action: WarningAction::AddBuy(Some(BuyPrefill {
                ticker: ticker.to_owned(),
                provider: provider.to_owned(),
                currency,
                first_sale,
            })),
        });
    } else if !uncovered.is_empty() {
        let sold = uncovered
            .iter()
            .map(|(ticker, _, _, shares, _)| format!("{ticker} {}", shares_text(*shares)))
            .collect::<Vec<_>>();
        cards.push(WarningCard {
            icon: IconName::TriangleAlert,
            title: format!("Shares sold with no buy: {}", sold.join(" · ")).into(),
            description: "They count with cost 0.00, so Costs (C-23) are too low. Import the \
                          earlier statements or add the buys."
                .into(),
            action: WarningAction::AddBuy(None),
        });
    }
    if !missing.is_empty() {
        let (verb, pronoun) = if missing.len() == 1 {
            ("has", "it")
        } else {
            ("have", "them")
        };
        cards.push(WarningCard {
            icon: IconName::MapPin,
            title: format!("{} {verb} no exchange", names(&missing)).into(),
            description: format!("PIT/ZG can't put {pronoun} in a country.").into(),
            action: WarningAction::SetExchange(rows),
        });
    }
    cards
}

fn missing_row(
    tax: &StockTaxData,
    ticker: &str,
    provider: &str,
    sales: usize,
    year: i32,
) -> ExchangeRow {
    let mut currencies = tax
        .sales
        .iter()
        .filter(|sale| sale.ticker == ticker && sale.provider == provider)
        .map(|sale| sale.price.currency.to_string())
        .collect::<Vec<_>>();
    currencies.sort();
    currencies.dedup();
    ExchangeRow {
        ticker: ticker.to_owned().into(),
        provider: provider.to_owned().into(),
        detail: format!(
            "{} · {provider} · {} in {year}",
            currencies.join(", "),
            counted(sales, "sale", "sales")
        )
        .into(),
        exchange: None,
    }
}
