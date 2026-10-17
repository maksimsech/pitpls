use crate::{
    components::records::RowDisplay,
    format::{DisplayText, amount, money, pln, shares},
    i18n::plural,
    pages::{
        records::day,
        stocks::{
            display::{
                Action, InstrumentKey, lots_count, price_text, rate_cell, ratio, shares_count,
                shares_in_lots, shares_text, trade_date,
            },
            set_exchange::ExchangeRow,
        },
    },
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use gpui_kit::SharedString;
use pitpls_app::use_case::stock::StockHistory;
use pitpls_core::{
    common::{Amount, Currency},
    stock::{
        CalculatedSale, Holding, LotMatch, OpenLot, SplitStep, Stock, StockAction, StockBuy,
        StockSplit, sort_key,
    },
    time,
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_i18n::t;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Range,
};

#[derive(Clone, Copy, PartialEq)]
pub enum Shade {
    First,
    Second,
    Open,
}

pub struct BarPart {
    pub weight: f32,
    pub label: SharedString,
    pub tooltip: SharedString,
    pub shade: Shade,
}

pub struct LotBar {
    pub bought: SharedString,
    pub label: SharedString,
    /// Of the bar of the largest lot.
    pub width: f32,
    pub parts: Vec<BarPart>,
    pub status: SharedString,
}

pub struct Fact {
    pub label: SharedString,
    pub value: SharedString,
    pub warning: bool,
}

pub struct YearGroup {
    pub year: i32,
    pub counts: SharedString,
    pub profit: Option<DisplayText>,
    pub rows: Range<usize>,
}

pub struct HistoryDisplay {
    pub instrument: ExchangeRow,
    pub facts: Vec<Fact>,
    pub lots_caption: SharedString,
    pub lots: Vec<LotBar>,
    pub legend: Vec<(Shade, SharedString)>,
    pub timeline_caption: SharedString,
    pub rows: Vec<RowDisplay>,
    pub notes: Vec<Vec<SharedString>>,
    pub years: Vec<YearGroup>,
    pub carried: Vec<SharedString>,
}

/// A lot as it stands on 31.12, or when its last shares were sold.
struct LotState<'a> {
    shares: Decimal,
    open: Decimal,
    price: Amount,
    nbp_rate: Decimal,
    nbp_date: NaiveDate,
    cost: Decimal,
    fee: Decimal,
    splits: &'a [SplitStep],
}

struct Replayed<'a> {
    open_lots: &'a [OpenLot],
    taken: Vec<(&'a CalculatedSale, &'a LotMatch)>,
}

impl<'a> Replayed<'a> {
    fn lot(&self, buy_id: &str) -> Option<LotState<'a>> {
        if let Some(lot) = self.open_lots.iter().find(|lot| lot.buy_id == buy_id) {
            return Some(LotState {
                shares: lot.lot_shares,
                open: lot.open,
                price: lot.price,
                nbp_rate: lot.nbp_rate,
                nbp_date: lot.nbp_date,
                cost: lot.lot_cost,
                fee: lot.lot_fee,
                splits: &lot.splits,
            });
        }
        let (_, lot) = self
            .taken
            .iter()
            .rev()
            .find(|(_, lot)| lot.buy_id == buy_id)?;
        Some(LotState {
            shares: lot.lot_shares,
            open: Decimal::ZERO,
            price: lot.price,
            nbp_rate: lot.nbp_rate,
            nbp_date: lot.nbp_date,
            cost: lot.lot_cost,
            fee: lot.lot_fee,
            splits: &lot.splits,
        })
    }

    fn taken_from(
        &self,
        buy_id: &str,
    ) -> impl Iterator<Item = &(&'a CalculatedSale, &'a LotMatch)> {
        self.taken
            .iter()
            .filter(move |(_, lot)| lot.buy_id == buy_id)
    }
}

impl HistoryDisplay {
    pub fn new(history: &StockHistory, key: &InstrumentKey, year: Option<i32>) -> Self {
        let replay = &history.replay;
        let mut records = history.records.iter().collect::<Vec<_>>();
        records.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
        let replayed = Replayed {
            open_lots: &replay.open_lots,
            taken: replay
                .sales
                .iter()
                .flat_map(|sale| sale.matches.iter().map(move |lot| (sale, lot)))
                .collect(),
        };
        let shades = replay
            .sales
            .iter()
            .filter(|sale| !sale.matches.is_empty())
            .enumerate()
            .map(|(index, sale)| {
                let shade = if index % 2 == 0 {
                    Shade::First
                } else {
                    Shade::Second
                };
                (sale.id.as_str(), shade)
            })
            .collect::<HashMap<_, _>>();

        let open = replay.open_lots.iter().map(|lot| lot.open).sum::<Decimal>();
        let holding = match replay.open_lots.len() {
            0 => t!("sale.no_shares").into_owned(),
            lots => shares_in_lots(open, lots),
        };
        let (exchange, warning) = match &history.listing {
            Some(listing) => (format!("{} · {}", listing.name, listing.country), false),
            None => (t!("stocks.no_exchange").into_owned(), true),
        };
        let currencies = records
            .iter()
            .filter_map(|record| match &record.action {
                StockAction::Buy(buy) => Some(buy.price.currency),
                StockAction::Sell(sell) => Some(sell.price.currency),
                StockAction::Split(_) => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|currency| currency.to_string())
            .collect::<Vec<_>>();
        let instrument = ExchangeRow {
            ticker: key.ticker.clone(),
            provider: key.provider.clone(),
            detail: if currencies.is_empty() {
                key.provider.clone()
            } else {
                format!("{} · {}", currencies.join(", "), key.provider).into()
            },
            exchange: history.listing.as_ref().map(|listing| listing.mic),
        };
        let facts = vec![
            Fact {
                label: if year.is_some() {
                    t!("history.exchange")
                } else {
                    t!("stocks.history_exchange")
                }
                .into(),
                value: exchange.into(),
                warning,
            },
            Fact {
                label: t!("history.currency").into(),
                value: if currencies.is_empty() {
                    "—".into()
                } else {
                    currencies.join(", ").into()
                },
                warning: false,
            },
            Fact {
                label: match year {
                    Some(year) => t!("history.holding_on", year = year),
                    None => t!("stocks.history_holding_today"),
                }
                .into(),
                value: holding.into(),
                warning: false,
            },
        ];

        let splits = records
            .iter()
            .filter(|record| matches!(record.action, StockAction::Split(_)))
            .count();
        let lots_caption = if splits == 0 {
            t!("history.lots_caption")
        } else {
            t!("history.lots_caption_split")
        };
        let timeline_caption = match year {
            Some(year) => {
                let caption = match splits {
                    0 => "history.timeline_caption",
                    1 => "history.timeline_caption_split",
                    _ => "history.timeline_caption_splits",
                };
                t!(
                    caption,
                    ticker = key.ticker,
                    provider = key.provider,
                    year = year
                )
            }
            None => {
                let caption = match splits {
                    0 => "stocks.history_timeline_all",
                    1 => "stocks.history_timeline_all_split",
                    _ => "stocks.history_timeline_all_splits",
                };
                t!(caption, ticker = key.ticker, provider = key.provider)
            }
        };

        let mut legend = replay
            .sales
            .iter()
            .filter_map(|sale| {
                let shade = shades.get(sale.id.as_str())?;
                Some((
                    *shade,
                    t!("history.sold", date = trade_date(sale.time)).into(),
                ))
            })
            .collect::<Vec<_>>();
        if let Some(year) = year.filter(|_| !open.is_zero()) {
            legend.push((
                Shade::Open,
                t!("history.open_moves", year = year + 1).into(),
            ));
        }

        let holdings = replay
            .holdings
            .iter()
            .map(|holding| (holding.record_id.as_str(), holding))
            .collect::<HashMap<_, _>>();
        let mut rows = Vec::with_capacity(records.len());
        let mut notes = Vec::with_capacity(records.len());
        let mut years: Vec<YearGroup> = Vec::new();
        let mut profits: BTreeMap<i32, Decimal> = BTreeMap::new();
        let mut counts: BTreeMap<i32, [usize; 3]> = BTreeMap::new();
        for record in &records {
            let (row, note) = event(
                record,
                &replayed,
                &replay.sales,
                holdings.get(record.id.as_str()),
            );
            let record_year = time::tax_date(record.time).year();
            let count = counts.entry(record_year).or_default();
            match &record.action {
                StockAction::Buy(_) => count[0] += 1,
                StockAction::Sell(_) => {
                    count[1] += 1;
                    // Profit goes to the tax year, which is the settlement year.
                    if let Some(sale) = replay.sales.iter().find(|sale| sale.id == record.id) {
                        *profits.entry(sale.settlement_date.year()).or_default() += sale.profit;
                    }
                }
                StockAction::Split(_) => count[2] += 1,
            }
            let index = rows.len();
            match years.last_mut() {
                Some(group) if group.year == record_year => group.rows.end = index + 1,
                _ => years.push(YearGroup {
                    year: record_year,
                    counts: SharedString::default(),
                    profit: None,
                    rows: index..index + 1,
                }),
            }
            rows.push(row);
            notes.push(note);
        }
        if year.is_none() {
            // A December trade can settle in a year without any trade records.
            for year in profits.keys() {
                if !years.iter().any(|group| group.year == *year) {
                    years.push(YearGroup {
                        year: *year,
                        counts: SharedString::default(),
                        profit: None,
                        rows: 0..0,
                    });
                }
            }
            years.sort_by_key(|group| group.year);
        }
        for group in &mut years {
            let [buys, sales, splits] = counts.get(&group.year).copied().unwrap_or_default();
            group.counts = [
                (buys, "count.buys"),
                (sales, "stocks.sales_count"),
                (splits, "count.splits"),
            ]
            .into_iter()
            .filter(|(count, _)| *count > 0)
            .map(|(count, key)| plural(key, count as u64))
            .collect::<Vec<_>>()
            .join(" · ")
            .into();
            group.profit = profits.get(&group.year).copied().map(pln);
        }

        let carried = replay
            .open_lots
            .iter()
            .map(|lot| {
                let (shares, date, price, cost) = (
                    shares_count(lot.open),
                    trade_date(lot.bought),
                    price_text(lot.price),
                    money(lot.cost).full,
                );
                if lot.fee_part.is_zero() {
                    t!(
                        "history.carried",
                        shares = shares,
                        date = date,
                        price = price,
                        cost = cost
                    )
                } else {
                    t!(
                        "history.carried_fee",
                        shares = shares,
                        date = date,
                        price = price,
                        cost = cost,
                        fee = money(lot.fee_part).full
                    )
                }
                .into()
            })
            .collect();

        Self {
            instrument,
            facts,
            lots_caption: lots_caption.into(),
            lots: lot_bars(&records, &replayed, &shades, year),
            legend,
            timeline_caption: timeline_caption.into(),
            rows,
            notes,
            years,
            carried,
        }
    }
}

fn weight(value: Decimal) -> f32 {
    value.to_f32().unwrap_or_default()
}

/// A sale before a later split took shares of the old count; its part is
/// scaled to the count after the splits, like the bar.
fn lot_bars(
    records: &[&Stock],
    replayed: &Replayed,
    shades: &HashMap<&str, Shade>,
    year: Option<i32>,
) -> Vec<LotBar> {
    let lots = records
        .iter()
        .filter_map(|record| match &record.action {
            StockAction::Buy(buy) => Some((*record, buy, replayed.lot(&record.id)?)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let widest = lots
        .iter()
        .map(|(_, _, lot)| lot.shares)
        .max()
        .unwrap_or(Decimal::ONE);

    lots.into_iter()
        .map(|(record, buy, lot)| {
            let mut parts = replayed
                .taken_from(&record.id)
                .map(|(sale, taken)| {
                    let scaled = taken.taken * lot.shares / taken.lot_shares;
                    let sold = trade_date(sale.time);
                    let tooltip = if scaled == taken.taken {
                        t!(
                            "history.part_sold",
                            shares = shares_count(taken.taken),
                            date = sold
                        )
                    } else {
                        t!(
                            "history.part_sold_split",
                            shares = shares_count(taken.taken),
                            date = sold,
                            scaled = shares_text(scaled)
                        )
                    };
                    BarPart {
                        weight: weight(scaled),
                        label: t!(
                            "history.part_label",
                            shares = shares_text(scaled),
                            date = sold
                        )
                        .into(),
                        tooltip: tooltip.into(),
                        shade: shades
                            .get(sale.id.as_str())
                            .copied()
                            .unwrap_or(Shade::First),
                    }
                })
                .collect::<Vec<_>>();
            if !lot.open.is_zero() {
                parts.push(BarPart {
                    weight: weight(lot.open),
                    label: t!("history.part_open", shares = shares_text(lot.open)).into(),
                    tooltip: match year {
                        Some(year) => t!(
                            "history.part_open_tooltip",
                            shares = shares_count(lot.open),
                            year = year,
                            next = year + 1
                        ),
                        None => t!(
                            "stocks.history_open_today_tooltip",
                            shares = shares_count(lot.open)
                        ),
                    }
                    .into(),
                    shade: Shade::Open,
                });
            }
            let bought = t!(
                "history.lot_at",
                shares = shares_text(buy.number),
                price = money(buy.price.value).full
            );
            let label = if lot.splits.is_empty() {
                bought
            } else {
                t!(
                    "history.lot_split_to",
                    bought = bought,
                    shares = shares_text(lot.shares),
                    price = money(lot.price.value).full
                )
            };
            let status = if lot.open.is_zero() {
                t!("history.lot_closed", shares = shares_text(lot.shares))
            } else {
                t!(
                    "history.lot_open",
                    shares = shares_text(lot.shares),
                    open = shares_text(lot.open)
                )
            };
            LotBar {
                bought: trade_date(record.time).into(),
                label: label.into(),
                width: weight(lot.shares / widest),
                parts,
                status: status.into(),
            }
        })
        .collect()
}

fn dash() -> DisplayText {
    DisplayText::plain("—")
}

fn signed(sign: &str, value: DisplayText) -> DisplayText {
    DisplayText {
        main: format!("{sign}{}", value.main).into(),
        full: format!("{sign}{}", value.full).into(),
        ..value
    }
}

fn tag(action: Action, detail: &str) -> DisplayText {
    DisplayText {
        extra: detail.to_owned().into(),
        ..DisplayText::plain(action.title())
    }
}

fn holding_cell(holding: Option<&&Holding>) -> DisplayText {
    match holding {
        None => dash(),
        Some(holding) if holding.lots == 0 => shares(holding.shares),
        Some(holding) => {
            let value = shares(holding.shares);
            let unit = format!("· {}", lots_count(holding.lots));
            DisplayText {
                full: format!("{} {unit}", value.full).into(),
                unit: Some(unit.into()),
                ..value
            }
        }
    }
}

fn event(
    record: &Stock,
    replayed: &Replayed,
    sales: &[CalculatedSale],
    holding: Option<&&Holding>,
) -> (RowDisplay, Vec<SharedString>) {
    let (action, cells, notes) = match &record.action {
        StockAction::Buy(buy) => {
            let lot = replayed.lot(&record.id);
            let cells = vec![
                day(record.time),
                tag(Action::Buy, &t!("history.new_lot")),
                signed("+", shares(buy.number)),
                amount(buy.price),
                lot.as_ref().map_or_else(dash, |lot| {
                    rate_cell(lot.nbp_rate, lot.nbp_date, buy.price.currency)
                }),
                lot.as_ref().map_or_else(dash, |lot| money(lot.cost)),
                dash(),
                holding_cell(holding),
            ];
            let fee = lot.as_ref().map_or(buy.fee.value, |lot| lot.fee);
            (Action::Buy, cells, fee_note(buy, fee).into_iter().collect())
        }
        StockAction::Sell(sell) => {
            let sale = sales.iter().find(|sale| sale.id == record.id);
            let cells = vec![
                day(record.time),
                tag(Action::Sell, ""),
                signed("−", shares(sell.number)),
                amount(sell.price),
                sale.map_or_else(dash, |sale| {
                    rate_cell(sale.nbp_rate, sale.nbp_date, sell.price.currency)
                }),
                sale.map_or_else(dash, |sale| money(sale.income)),
                sale.map_or_else(dash, |sale| money(sale.profit)),
                holding_cell(holding),
            ];
            (
                Action::Sell,
                cells,
                sale.map(sale_note).into_iter().collect(),
            )
        }
        StockAction::Split(split) => {
            let cells = vec![
                day(record.time),
                tag(Action::Split, &ratio(split.before, split.after)),
                DisplayText::plain(split_factor(split)),
                dash(),
                dash(),
                dash(),
                dash(),
                holding_cell(holding),
            ];
            (Action::Split, cells, split_notes(record, replayed))
        }
    };
    let row = RowDisplay {
        id: record.id.clone().into(),
        time: record.time,
        cells,
        label: format!(
            "{} {} {}",
            action.title(),
            record.ticker,
            trade_date(record.time)
        )
        .into(),
        ..Default::default()
    };
    (row, notes)
}

fn split_factor(split: &StockSplit) -> String {
    match (split.before, split.after) {
        (1, after) => format!("×{after}"),
        (before, 1) => format!("÷{before}"),
        (before, after) => format!("×{after}/{before}"),
    }
}

fn fee_note(buy: &StockBuy, fee_pln: Decimal) -> Option<SharedString> {
    if buy.fee.value.is_zero() {
        return None;
    }
    let note = if buy.fee.currency == Currency::PLN {
        t!("history.fee_note_pln", pln = money(fee_pln).full)
    } else {
        t!(
            "history.fee_note",
            fee = price_text(buy.fee),
            pln = money(fee_pln).full
        )
    };
    Some(note.into())
}

/// Every part, unlike `and_list`, which shortens long lists.
fn join_and(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => t!("list.and", items = rest.join(", "), last = last).into_owned(),
    }
}

fn sale_note(sale: &CalculatedSale) -> SharedString {
    let mut parts = sale
        .matches
        .iter()
        .map(|lot| {
            let date = trade_date(lot.bought);
            let taken = shares_text(lot.taken);
            if !lot.after.is_zero() {
                let before = shares_text(lot.before);
                t!(
                    "history.took_part",
                    taken = taken,
                    before = before,
                    date = date
                )
            } else if lot.before == lot.lot_shares {
                t!("history.took_all", taken = taken, date = date)
            } else {
                t!("history.took_last", taken = taken, date = date)
            }
            .into_owned()
        })
        .collect::<Vec<_>>();
    if sale.uncovered > Decimal::ZERO {
        parts.push(t!("history.took_no_buy", shares = shares_text(sale.uncovered)).into_owned());
    }
    let settles = sale.settlement_date.year();
    let (parts, costs) = (join_and(&parts), money(sale.costs).full);
    if settles == time::tax_date(sale.time).year() {
        t!("history.took", parts = parts, costs = costs)
    } else {
        t!(
            "history.took_counts_in",
            parts = parts,
            costs = costs,
            year = settles
        )
    }
    .into()
}

/// A lot the split changed is still open on 31.12 or was sold from after it.
fn split_notes(record: &Stock, replayed: &Replayed) -> Vec<SharedString> {
    let mut changed: BTreeMap<(DateTime<Utc>, &str), (&SplitStep, Decimal)> = BTreeMap::new();
    let open = replayed
        .open_lots
        .iter()
        .map(|lot| (lot.bought, lot.buy_id.as_str(), &lot.splits, lot.lot_cost));
    let taken = replayed
        .taken
        .iter()
        .map(|(_, lot)| (lot.bought, lot.buy_id.as_str(), &lot.splits, lot.lot_cost));
    for (bought, buy_id, splits, cost) in open.chain(taken) {
        if let Some(step) = splits.iter().find(|step| step.split_id == record.id) {
            changed.entry((bought, buy_id)).or_insert((step, cost));
        }
    }
    if changed.is_empty() {
        return vec![t!("sale.no_open_lots").into()];
    }
    changed
        .into_iter()
        .map(|((bought, _), (step, cost))| {
            t!(
                "history.split_lot",
                date = trade_date(bought),
                before = shares_text(step.lot_shares_before),
                price_before = money(step.price_before.value).full,
                after = shares_text(step.lot_shares_after),
                price_after = price_text(step.price_after),
                cost = money(cost).full
            )
            .into()
        })
        .collect()
}
