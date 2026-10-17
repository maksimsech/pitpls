use super::set_exchange::ExchangeRow;
use crate::{
    components::records::{EntryStyle, LotRow, LotTable, RowDisplay, Step, StepLine},
    format::{DisplayText, amount, date, money, pln, shares},
    i18n::{and_list, plural, plural_key, plural_key_decimal},
    pages::records::{date_label, day},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use gpui_kit::{SharedString, assets::IconName};
use pitpls_app::use_case::stock::{Listing, Listings, StockData};
use pitpls_core::{
    common::{Amount, Currency},
    stock::{
        CalculatedSale, Holding, LotMatch, OpenLot, Replay, SplitStep, Stock, StockAction,
        StockBuy, StockSell, StockSplit, StockTaxData, Warning,
    },
    time,
};
use rust_decimal::Decimal;
use rust_i18n::t;
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Buy,
    Sell,
    Split,
}

impl Action {
    pub fn title(self) -> Cow<'static, str> {
        match self {
            Self::Buy => t!("action.buy"),
            Self::Sell => t!("action.sell"),
            Self::Split => t!("action.split"),
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
    pub portfolio: Option<super::portfolio::PortfolioDisplay>,
    /// The labels are message keys.
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
    pub fn all_years(data: &pitpls_app::use_case::portfolio::StockPortfolio) -> Self {
        let replay = ReplayView::from(&data.replay);
        let mut stocks = data.records.clone();
        stocks.sort_by(|a, b| (b.time, &b.id).cmp(&(a.time, &a.id)));
        let mut histories = HashMap::new();
        let records = stocks
            .iter()
            .map(|record| {
                let keys = history_keys(record, replay.holdings);
                let row = record_row(record, &keys, &replay, &data.listings, None);
                histories.insert(row.id.clone(), keys);
                row
            })
            .collect();
        Self {
            portfolio: Some(super::portfolio::PortfolioDisplay::new(data)),
            stats: Vec::new(),
            warnings: Vec::new(),
            sales: Vec::new(),
            profits: Vec::new(),
            records,
            actions: stocks.iter().map(action).collect(),
            stocks,
            holdings: Vec::new(),
            sold_out: Vec::new(),
            histories,
        }
    }

    pub fn new(data: &StockData, year: i32) -> Self {
        let tax = &data.tax;
        let listings = &data.listings;
        let replay = ReplayView::from(tax);
        let mut histories = HashMap::new();
        let sales = tax
            .sales
            .iter()
            .map(|sale| {
                let row = sale_row(sale, holding_after(&tax.holdings, sale), listings);
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
                let keys = history_keys(record, &tax.holdings);
                let row = record_row(record, &keys, &replay, listings, Some(year));
                histories.insert(row.id.clone(), keys);
                row
            })
            .collect();
        let (holdings, sold_out) = holdings(tax, listings);
        Self {
            portfolio: None,
            stats: vec![
                ("tax.income_c22", pln(tax.income)),
                ("tax.costs_c23", pln(tax.costs)),
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

fn holding_after<'a>(holdings: &'a [Holding], sale: &CalculatedSale) -> Option<&'a Holding> {
    holdings
        .iter()
        .find(|holding| holding.record_id == sale.id && holding.provider == sale.provider)
}

/// A split has a history in every provider that held or traded the ticker by
/// then.
fn history_keys(record: &Stock, holdings: &[Holding]) -> Vec<InstrumentKey> {
    match record.provider() {
        Some(provider) => vec![InstrumentKey::new(&record.ticker, provider)],
        None => holdings
            .iter()
            .filter(|holding| holding.record_id == record.id)
            .map(|holding| InstrumentKey::new(&record.ticker, &holding.provider))
            .collect(),
    }
}

fn history_links(ticker: &str, keys: &[InstrumentKey]) -> Vec<SharedString> {
    match keys {
        [_] => vec![t!("sale.history", ticker = ticker).into()],
        keys => keys
            .iter()
            .map(|key| t!("sale.history_in", ticker = ticker, provider = key.provider).into())
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
    let key = plural_key_decimal("count.shares", value);
    t!(key, count = shares_text(value)).into_owned()
}

pub fn lots_count(lots: usize) -> String {
    plural("count.lots", lots as u64).into_owned()
}

/// "5 shares in 2 lots"; Polish puts the lots in another case.
pub fn shares_in_lots(value: Decimal, lots: usize) -> String {
    t!(
        "sale.shares_in_lots",
        shares = shares_count(value),
        lots = plural("count.in_lots", lots as u64)
    )
    .into_owned()
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
    t!("sale.ratio", after = after, before = before).into_owned()
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

// `total` is the provider's total with its name, when the value is that and not shares × price.
pub fn conversion(
    shares_value: Decimal,
    price: Amount,
    total: Option<(Amount, &str)>,
    rate: Decimal,
) -> Vec<DisplayText> {
    let mut parts = match total {
        Some((total, provider)) => vec![
            amount(total),
            DisplayText::plain(t!("sale.provider_total", provider = provider)),
        ],
        None => vec![shares(shares_value), DisplayText::plain("×"), amount(price)],
    };
    if price.currency != Currency::PLN {
        parts.push(DisplayText::plain(format!("× {rate}")));
    }
    parts
}

pub fn total_note(provider: &str, currency: Currency) -> String {
    t!("sale.total_note", provider = provider, currency = currency).into_owned()
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
        || DisplayText::plain(t!("sale.missing")),
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
    let link = t!("sale.full_history", ticker = sale.ticker);
    RowDisplay {
        id: sale.id.clone().into(),
        time: sale.time,
        cells,
        steps: sale_steps(sale, listing, true),
        lots: Some(fifo_table(sale, holding, Some(link.into()))),
        flagged: if exchange.is_none() { vec![2] } else { vec![] },
        search: search(
            &sale.ticker,
            &sale.provider,
            exchange.as_ref(),
            &t!("action.sell").to_lowercase(),
        ),
        label: t!(
            "sale.label",
            ticker = sale.ticker,
            date = date_label(sale.time)
        )
        .into(),
        links: vec![],
    }
}

fn search(ticker: &str, provider: &str, exchange: Option<&SharedString>, action: &str) -> String {
    let exchange = exchange.map_or("", |exchange| exchange.as_ref());
    format!("{ticker}\n{provider}\n{exchange}\n{action}").to_lowercase()
}

fn sale_steps(sale: &CalculatedSale, listing: Option<&Listing>, tax: bool) -> Vec<Step> {
    let settled = if sale.settlement_worked_out {
        t!(
            "sale.settled_worked_out",
            date = date(sale.settlement_date).main
        )
    } else {
        t!("sale.settled", date = date(sale.settlement_date).main)
    };
    let nbp = if sale.price.currency == Currency::PLN {
        t!("sale.no_conversion")
    } else {
        t!("records.nbp_date", date = date(sale.nbp_date).main)
    };
    let counts_in = counts_in(sale);

    let mut costs = vec![
        entry(
            t!("sale.lots_taken"),
            vec![pln(sale.lots_cost)],
            EntryStyle::Normal,
        ),
        entry(
            t!("sale.buy_fees_taken"),
            vec![pln(sale.buy_fees)],
            EntryStyle::Normal,
        ),
    ];
    if sale.uncovered > Decimal::ZERO {
        costs.push(entry(
            t!("sale.no_buy_shares", shares = shares_count(sale.uncovered)),
            vec![pln(Decimal::ZERO)],
            EntryStyle::Warning,
        ));
    }
    costs.extend([
        entry(
            fee_label(
                &t!("sale.sale_fee"),
                sale.fee,
                sale.fee_pln,
                sale.price,
                sale.nbp_rate,
            ),
            vec![pln(sale.fee_pln)],
            EntryStyle::Normal,
        ),
        entry(
            t!("column.costs"),
            vec![pln(sale.costs)],
            EntryStyle::Strong,
        ),
    ]);
    if tax {
        costs.push(StepLine::Caption(
            match counts_in {
                Some(year) => t!("sale.adds_to_costs_in", year = year),
                None => t!("sale.adds_to_costs"),
            }
            .into(),
        ));
    }

    let result = if sale.profit < Decimal::ZERO {
        t!("home.loss")
    } else {
        t!("home.profit")
    };
    let tax_paid_pln = money(sale.tax_paid_pln).full;
    let result_caption = if !tax {
        t!("stocks.counts_in_year", year = sale.settlement_date.year())
    } else {
        match (sale.country, listing) {
            (Some(country), Some(listing)) => t!(
                "sale.pit_zg",
                country = country,
                exchange = listing.name,
                tax = tax_paid_pln
            ),
            _ if sale.tax_paid.is_some() => t!("sale.pit_zg_no_country_tax", tax = tax_paid_pln),
            _ => t!("sale.pit_zg_no_country"),
        }
    };
    let adds_to_income = match counts_in {
        Some(year) => t!("sale.adds_to_income_in", year = year),
        None => t!("sale.adds_to_income"),
    };

    let mut income = vec![
        StepLine::Formula(conversion(
            sale.shares,
            sale.price,
            sale.from_total
                .then_some((sale.value, sale.provider.as_str())),
            sale.nbp_rate,
        )),
        StepLine::Result(vec![pln(sale.income)]),
        StepLine::Caption(
            if tax {
                format!("{settled} · {nbp} · {adds_to_income}")
            } else {
                format!("{settled} · {nbp}")
            }
            .into(),
        ),
    ];
    if sale.from_total {
        income.push(StepLine::Caption(
            total_note(&sale.provider, sale.value.currency).into(),
        ));
    }

    vec![
        Step {
            title: "column.income",
            lines: income,
        },
        Step {
            title: "column.costs",
            lines: costs,
        },
        Step {
            title: "sale.result",
            lines: vec![
                StepLine::Formula(vec![
                    money(sale.income),
                    DisplayText::plain("−"),
                    money(sale.costs),
                ]),
                StepLine::Result(vec![DisplayText::plain(result), pln(sale.profit.abs())]),
                StepLine::Caption(result_caption.into()),
            ],
        },
    ]
}

struct ReplayView<'a> {
    sales: &'a [CalculatedSale],
    next_year_sales: &'a [CalculatedSale],
    open_lots: &'a [OpenLot],
    holdings: &'a [Holding],
}

impl<'a> From<&'a StockTaxData> for ReplayView<'a> {
    fn from(tax: &'a StockTaxData) -> Self {
        Self {
            sales: &tax.sales,
            next_year_sales: &tax.next_year_sales,
            open_lots: &tax.open_lots,
            holdings: &tax.holdings,
        }
    }
}

impl<'a> From<&'a Replay> for ReplayView<'a> {
    fn from(replay: &'a Replay) -> Self {
        Self {
            sales: &replay.sales,
            next_year_sales: &[],
            open_lots: &replay.open_lots,
            holdings: &replay.holdings,
        }
    }
}

impl ReplayView<'_> {
    fn all_sales(&self) -> impl Iterator<Item = &CalculatedSale> {
        self.sales.iter().chain(self.next_year_sales)
    }
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
        t!("sale.fee_in_pln", fee = name).into_owned()
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
        0 => t!("sale.no_shares").into_owned(),
        lots => shares_in_lots(after + taken, lots),
    };
    let caption = if sale.uncovered > Decimal::ZERO {
        t!(
            "sale.before_sold",
            before = before,
            sold = shares_count(sale.shares)
        )
    } else if after_lots == 0 {
        t!("sale.before_closed", before = before)
    } else {
        t!(
            "sale.before_after",
            before = before,
            after = shares_text(after),
            lots = plural("count.in_lots", after_lots as u64)
        )
    };

    let dash = || DisplayText::plain("—");
    let mut rows = sale.matches.iter().map(lot_match_row).collect::<Vec<_>>();
    if sale.uncovered > Decimal::ZERO {
        rows.push(LotRow {
            cells: vec![
                DisplayText::plain(t!("sale.no_buy")),
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
        title: t!(
            "sale.fifo_title",
            ticker = sale.ticker,
            provider = sale.provider
        )
        .into(),
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
    t!(
        "sale.split_note",
        ratio = ratio(split.before, split.after),
        date = trade_date(split.time),
        before = shares_text(split.lot_shares_before),
        price_before = money(split.price_before.value).full,
        after = shares_text(split.lot_shares_after),
        price_after = price_text(split.price_after)
    )
    .into()
}

fn record_row(
    record: &Stock,
    histories: &[InstrumentKey],
    replay: &ReplayView<'_>,
    listings: &Listings,
    year: Option<i32>,
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
                if year.is_none() && exchange.is_none() {
                    dash()
                } else {
                    exchange_cell(exchange.as_ref())
                },
                DisplayText::plain(provider.clone()),
                shares(*number),
                amount(*price),
                record.value().map_or_else(dash, amount),
                amount(*fee),
            ];
            let (steps, lots) = match &record.action {
                StockAction::Buy(buy) => (buy_steps(record, buy, replay, year), None),
                _ => match replay.all_sales().find(|sale| sale.id == record.id) {
                    Some(sale) => {
                        let listing = listings
                            .get(&sale.ticker, &sale.provider)
                            .filter(|_| sale.exchange.is_some());
                        (
                            sale_steps(sale, listing, year.is_some()),
                            Some(fifo_table(sale, holding_after(replay.holdings, sale), None)),
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
                DisplayText::plain(t!("sale.all_providers")),
                DisplayText::plain(format!("{} → {}", split.before, split.after)),
                dash(),
                dash(),
                dash(),
            ];
            (cells, None, split_steps(record, split, replay), None)
        }
    };
    let flagged = if year.is_some() && exchange.is_none() && action != Action::Split {
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
    value: Amount,
    from_total: bool,
    nbp_rate: Decimal,
    nbp_date: NaiveDate,
    cost: Decimal,
    fee: Decimal,
    splits: &'a [SplitStep],
}

fn buy_steps(
    record: &Stock,
    buy: &StockBuy,
    replay: &ReplayView<'_>,
    year: Option<i32>,
) -> Vec<Step> {
    let taken = replay
        .all_sales()
        .flat_map(|sale| {
            sale.matches
                .iter()
                .filter(|lot| lot.buy_id == record.id)
                .map(move |lot| (sale, lot))
        })
        .collect::<Vec<_>>();
    let open = replay.open_lots.iter().find(|lot| lot.buy_id == record.id);
    let lot = match (open, taken.last()) {
        (Some(open), _) => Some(LotView {
            value: open.lot_value,
            from_total: open.from_total,
            nbp_rate: open.nbp_rate,
            nbp_date: open.nbp_date,
            cost: open.lot_cost,
            fee: open.lot_fee,
            splits: &open.splits,
        }),
        (None, Some((_, lot))) => Some(LotView {
            value: lot.lot_value,
            from_total: lot.from_total,
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
        t!("sale.no_conversion")
    } else {
        t!("records.nbp_date", date = date(lot.nbp_date).main)
    };
    let fee_formula = if buy.fee.value.is_zero() {
        vec![DisplayText::plain(t!("sale.no_fee"))]
    } else if buy.fee.currency == Currency::PLN {
        vec![amount(buy.fee)]
    } else {
        conversion_fee(buy.fee, fee_rate(buy.fee, lot.fee, buy.price, lot.nbp_rate))
    };
    let fee_caption = if buy.fee.currency == Currency::PLN && !buy.fee.value.is_zero() {
        t!("sale.fee_counted_pln")
    } else {
        t!("sale.fee_counted")
    };

    let mut events = vec![(
        record.time,
        entry(
            t!("sale.bought", date = trade_date(record.time)),
            vec![shares(buy.number)],
            EntryStyle::Normal,
        ),
    )];
    events.extend(lot.splits.iter().map(|split| {
        (
            split.time,
            entry(
                t!(
                    "sale.split_event",
                    ratio = ratio(split.before, split.after),
                    date = trade_date(split.time)
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
                    Some(year) => {
                        t!(
                            "sale.sold_counts_in",
                            date = trade_date(sale.time),
                            year = year
                        )
                    }
                    None => t!("sale.sold", date = trade_date(sale.time)),
                },
                vec![
                    shares(lot.taken),
                    DisplayText::plain(t!("sale.to_costs")),
                    added(lot.cost + lot.fee_part),
                ],
                EntryStyle::Normal,
            ),
        )
    }));
    events.sort_by_key(|(time, _)| *time);
    let open_shares = open.map_or(Decimal::ZERO, |lot| lot.open);
    let mut open_value = vec![shares(open_shares)];
    if let Some(year) = year.filter(|_| !open_shares.is_zero()) {
        open_value.push(DisplayText::plain(t!("sale.moves_into", year = year + 1)));
    }
    let mut lines = events.into_iter().map(|(_, line)| line).collect::<Vec<_>>();
    lines.push(entry(
        match year {
            Some(year) => t!("sale.open_on", year = year),
            None => t!("stocks.open_today"),
        },
        open_value,
        EntryStyle::Strong,
    ));

    let mut cost = vec![
        StepLine::Formula(conversion(
            buy.number,
            buy.price,
            lot.from_total.then_some((lot.value, buy.provider.as_str())),
            lot.nbp_rate,
        )),
        StepLine::Result(vec![pln(lot.cost)]),
        StepLine::Caption(format!("{nbp} · {}", t!("sale.cost_when_sold")).into()),
    ];
    if lot.from_total {
        cost.push(StepLine::Caption(
            total_note(&buy.provider, lot.value.currency).into(),
        ));
    }

    vec![
        Step {
            title: "sale.cost",
            lines: cost,
        },
        Step {
            title: "field.fee",
            lines: vec![
                StepLine::Formula(fee_formula),
                StepLine::Result(vec![pln(lot.fee)]),
                StepLine::Caption(fee_caption.into()),
            ],
        },
        Step {
            title: "sale.lot_history",
            lines,
        },
    ]
}

fn conversion_fee(fee: Amount, rate: Decimal) -> Vec<DisplayText> {
    vec![amount(fee), DisplayText::plain(format!("× {rate}"))]
}

fn split_steps(record: &Stock, split: &StockSplit, replay: &ReplayView<'_>) -> Vec<Step> {
    let mut changed: BTreeMap<(&str, DateTime<Utc>, &str), &SplitStep> = BTreeMap::new();
    let taken = replay.all_sales().flat_map(|sale| {
        sale.matches.iter().map(move |lot| {
            (
                sale.provider.as_str(),
                lot.bought,
                lot.buy_id.as_str(),
                &lot.splits,
            )
        })
    });
    let open = replay.open_lots.iter().map(|lot| {
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
                t!(
                    "sale.provider_lot",
                    provider = provider,
                    date = trade_date(bought)
                ),
                vec![DisplayText::plain(t!(
                    "sale.split_change",
                    before = shares_text(step.lot_shares_before),
                    price_before = money(step.price_before.value).full,
                    after = shares_text(step.lot_shares_after),
                    price_after = price_text(step.price_after)
                ))],
                EntryStyle::Normal,
            )
        })
        .collect::<Vec<_>>();
    if lots.is_empty() {
        lots.push(StepLine::Caption(t!("sale.no_open_lots").into()));
    } else {
        lots.push(StepLine::Caption(t!("sale.split_costs_note").into()));
    }

    vec![
        Step {
            title: "action.split",
            lines: vec![
                StepLine::Formula(vec![DisplayText::plain(t!(
                    "sale.split_formula",
                    after = split.after,
                    before = split.before
                ))]),
                StepLine::Result(vec![DisplayText::plain(ratio(split.before, split.after))]),
                StepLine::Caption(t!("sale.split_scope", ticker = record.ticker).into()),
            ],
        },
        Step {
            title: "sale.lots_changed",
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
    for sale in tax.sales.iter().chain(&tax.next_year_sales) {
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
                last_sale: t!("sale.last_sale", date = trade_date(sale.time)).into(),
                search: search(ticker, provider, exchange.as_ref(), ""),
            }
        })
        .collect();
    (blocks, sold_out)
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
            title: t!(
                "sale.warning_no_buy",
                ticker = ticker,
                shares = shares_count(shares)
            )
            .into(),
            description: t!("sale.warning_no_buy_detail", provider = provider).into(),
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
            title: t!("sale.warning_no_buys", sales = sold.join(" · ")).into(),
            description: t!("sale.warning_no_buys_detail").into(),
            action: WarningAction::AddBuy(None),
        });
    }
    if !missing.is_empty() {
        let count = missing.len() as u64;
        cards.push(WarningCard {
            icon: IconName::MapPin,
            title: t!(
                plural_key("sale.warning_no_exchange", count),
                tickers = and_list(&missing)
            )
            .into(),
            description: t!(plural_key("sale.warning_no_exchange_detail", count)).into(),
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
            "{} · {provider} · {}",
            currencies.join(", "),
            t!(
                "sale.sales_in_year",
                sales = plural("stocks.sales_count", sales as u64),
                year = year
            )
        )
        .into(),
        exchange: None,
    }
}

#[cfg(test)]
mod tests {
    use pitpls_app::use_case::stock::load_stocks;
    use rust_decimal::dec;

    use super::*;
    use crate::fixtures::{self, REVOLUT, T212, buy, on, sell, split, usd};

    // AAPL and MSFT have an exchange; KO sells a share it never bought; NVDA splits in two
    // providers.
    async fn display() -> StockDisplay {
        let app = fixtures::app(2025..=2025).await;
        fixtures::listed(&app, "AAPL", T212, "XNAS").await;
        fixtures::listed(&app, "MSFT", T212, "XNAS").await;
        fixtures::add_stocks(
            &app,
            &[
                buy(
                    "aapl-1",
                    on(2025, 2, 3),
                    "AAPL",
                    T212,
                    dec!(10),
                    usd(dec!(100)),
                ),
                buy(
                    "aapl-2",
                    on(2025, 3, 3),
                    "AAPL",
                    T212,
                    dec!(5),
                    usd(dec!(120)),
                ),
                sell(
                    "aapl-s",
                    on(2025, 5, 5),
                    "AAPL",
                    T212,
                    dec!(12),
                    usd(dec!(150)),
                ),
                buy(
                    "ko-1",
                    on(2025, 2, 3),
                    "KO",
                    REVOLUT,
                    dec!(2),
                    usd(dec!(50)),
                ),
                sell(
                    "ko-s",
                    on(2025, 6, 2),
                    "KO",
                    REVOLUT,
                    dec!(3),
                    usd(dec!(60)),
                ),
                buy(
                    "msft-1",
                    on(2025, 2, 3),
                    "MSFT",
                    T212,
                    dec!(1),
                    usd(dec!(300)),
                ),
                sell(
                    "msft-s",
                    on(2025, 4, 1),
                    "MSFT",
                    T212,
                    dec!(1),
                    usd(dec!(310)),
                ),
                buy(
                    "nvda-t212",
                    on(2025, 2, 3),
                    "NVDA",
                    T212,
                    dec!(1),
                    usd(dec!(500)),
                ),
                buy(
                    "nvda-rev",
                    on(2025, 2, 3),
                    "NVDA",
                    REVOLUT,
                    dec!(2),
                    usd(dec!(500)),
                ),
                split("nvda-split", on(2025, 6, 10), "NVDA", 1, 10),
            ],
        )
        .await;
        let data = load_stocks(&app, 2025).await.unwrap();
        StockDisplay::new(&data, 2025)
    }

    fn row<'a>(rows: &'a [RowDisplay], id: &str) -> &'a RowDisplay {
        rows.iter()
            .find(|row| row.id.as_ref() == id)
            .unwrap_or_else(|| panic!("no row {id}"))
    }

    fn texts(parts: &[DisplayText]) -> Vec<&str> {
        parts.iter().map(|part| part.full.as_ref()).collect()
    }

    fn column(table: &LotTable, index: usize) -> Vec<&str> {
        table
            .rows
            .iter()
            .map(|row| row.cells[index].full.as_ref())
            .collect()
    }

    #[tokio::test]
    async fn sale_row_lists_the_lots_it_takes() {
        let display = display().await;

        assert_eq!(
            display
                .stats
                .iter()
                .map(|(key, value)| (*key, value.full.as_ref()))
                .collect::<Vec<_>>(),
            [
                ("tax.income_c22", "9\u{202f}160.00 PLN"),
                ("tax.costs_c23", "6\u{202f}560.00 PLN"),
            ]
        );
        let sale = row(&display.sales, "aapl-s");
        assert_eq!(
            texts(&sale.cells),
            [
                "05.05",
                "AAPL",
                "NASDAQ",
                "Trading 212",
                "12",
                "7\u{202f}200.00 PLN",
                "4\u{202f}960.00 PLN",
                "2\u{202f}240.00 PLN",
            ]
        );
        assert!(sale.flagged.is_empty());
        let lots = sale.lots.as_ref().unwrap();
        // Bought, lot shares, before, taken, after.
        assert_eq!(column(lots, 0), ["03.02.2025", "03.03.2025"]);
        assert_eq!(column(lots, 3), ["10", "2"]);
        assert_eq!(column(lots, 4), ["0", "3"]);
        assert!(lots.rows.iter().all(|row| !row.uncovered));
        assert_eq!(
            lots.caption.as_ref(),
            "15 shares in 2 lots before this sale → 3 in 1 lot after"
        );
    }

    #[tokio::test]
    async fn sale_without_a_buy_shows_the_missing_shares_and_exchange() {
        let display = display().await;

        let sale = row(&display.sales, "ko-s");
        assert_eq!(sale.flagged, [2]);
        let lots = sale.lots.as_ref().unwrap();
        assert_eq!(column(lots, 0), ["03.02.2025", "No buy"]);
        assert_eq!(column(lots, 3), ["2", "1"]);
        assert_eq!(
            lots.rows
                .iter()
                .map(|row| row.uncovered)
                .collect::<Vec<_>>(),
            [false, true]
        );
        assert_eq!(
            lots.caption.as_ref(),
            "2 shares in 1 lot before this sale, 3 shares sold"
        );
    }

    #[tokio::test]
    async fn warnings_ask_for_the_buy_and_the_exchange() {
        let display = display().await;

        let [no_buy, no_exchange] = &display.warnings[..] else {
            panic!("expected two warnings");
        };
        assert_eq!(no_buy.title.as_ref(), "KO sold 1 share with no buy");
        let WarningAction::AddBuy(Some(prefill)) = &no_buy.action else {
            panic!("expected a prefilled buy");
        };
        assert_eq!(
            (
                prefill.ticker.as_str(),
                prefill.provider.as_str(),
                prefill.currency,
                prefill.first_sale
            ),
            ("KO", REVOLUT, Currency::USD, on(2025, 6, 2))
        );
        let WarningAction::SetExchange(rows) = &no_exchange.action else {
            panic!("expected the exchange rows");
        };
        assert_eq!(
            rows.iter()
                .map(|row| (row.ticker.as_ref(), row.provider.as_ref()))
                .collect::<Vec<_>>(),
            [("KO", REVOLUT)]
        );
    }

    #[tokio::test]
    async fn holdings_show_open_lots_and_sold_out_instruments() {
        let display = display().await;

        assert_eq!(
            display
                .holdings
                .iter()
                .map(|block| (
                    block.ticker.as_ref(),
                    block.provider.as_ref(),
                    block.shares,
                    block.lots.len()
                ))
                .collect::<Vec<_>>(),
            [
                ("AAPL", T212, dec!(3), 1),
                ("NVDA", REVOLUT, dec!(20), 1),
                ("NVDA", T212, dec!(10), 1),
            ]
        );
        let aapl = &display.holdings[0].lots[0];
        // Bought, lot shares, open.
        assert_eq!(texts(&aapl.cells[..3]), ["03.03.2025", "5", "3"]);
        assert_eq!(
            display
                .sold_out
                .iter()
                .map(|sold| sold.instrument.as_ref())
                .collect::<Vec<_>>(),
            ["KO · Revolut", "MSFT · NASDAQ · Trading 212"]
        );
    }

    #[tokio::test]
    async fn split_links_the_history_of_every_provider() {
        let display = display().await;

        assert_eq!(
            row(&display.records, "nvda-split").links,
            ["NVDA history · Revolut", "NVDA history · Trading 212"]
        );
        assert_eq!(row(&display.records, "aapl-1").links, ["AAPL history"]);
        let providers = display.histories["nvda-split"]
            .iter()
            .map(|key| key.provider.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(providers, [REVOLUT, T212]);
    }
}
