use super::{
    StocksPage,
    display::{
        Action, BuyPrefill, conversion, fee_rate, price_text, ratio, shares_count, shares_in_lots,
        shares_text, total_note, trade_date,
    },
    exchange::{self, Catalog, ExchangeList, PickerState},
};
use crate::{
    components::{
        dialog,
        form::{self, ChoiceState},
        records::{preview_error, preview_frame, preview_value},
    },
    format::{DisplayText, date, money, pln},
    i18n::{and_list, plural_key_decimal},
    pages::records::{Submission, TableHost},
    theme::palette,
};
use chrono::{DateTime, NaiveDate, Utc};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    Role,
    assets::IconName,
    component::{
        button::*,
        collapsible::Collapsible,
        date_picker::{DatePicker, DatePickerState},
        dialog::Dialog,
        input::InputState,
        select::SelectEvent,
        *,
    },
    *,
};
use pitpls_app::use_case::stock::{
    self, CreateStockInput, Listings, StockActionFields, StockFields, TradeFields, UpdateStockInput,
};
use pitpls_core::{
    common::{Amount, Currency, Mic},
    stock::{
        CalculatedSale, Instrument, Stock, StockAction, StockBuy, StockPreview, StockSell,
        StockSplit,
    },
    time,
};
use rust_decimal::Decimal;
use rust_i18n::t;
use std::borrow::Cow;

type StockSubmission = Submission<CreateStockInput, UpdateStockInput>;

struct Known {
    instruments: Vec<Instrument>,
    listings: Listings,
}

#[derive(Clone, PartialEq)]
struct SettlesKey {
    time: DateTime<Utc>,
    ticker: String,
    provider: String,
    currency: Currency,
    exchange: Option<Mic>,
}

pub struct StockEditor {
    existing: Option<String>,
    action: Action,
    id: Entity<InputState>,
    time: Entity<DatePickerState>,
    day: Entity<DatePickerState>,
    ticker: Entity<InputState>,
    provider: Entity<InputState>,
    exchange: Entity<PickerState>,
    exchange_picked: bool,
    shares: Entity<InputState>,
    price: Entity<InputState>,
    price_currency: Entity<ChoiceState<Currency>>,
    kept_amount: Option<KeptAmount>,
    fee: Entity<InputState>,
    fee_currency: Entity<ChoiceState<Currency>>,
    tax_paid: Entity<InputState>,
    tax_currency: Entity<ChoiceState<Currency>>,
    settlement: Entity<DatePickerState>,
    before: Entity<InputState>,
    after: Entity<InputState>,
    id_open: bool,
    known: Option<Known>,
    filled: Option<usize>,
    draft: Option<Draft>,
    preview: Option<Result<PreviewView, SharedString>>,
    preview_task: Option<Task<()>>,
    settles_key: Option<SettlesKey>,
    settles: Option<NaiveDate>,
    settles_task: Option<Task<()>>,
    known_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

// An imported trade's amount holds only for the shares and price it came with.
#[derive(Clone, Copy)]
struct KeptAmount {
    shares: Decimal,
    price: (Decimal, Currency),
    amount: Decimal,
}

struct TradeValues<'a> {
    number: Decimal,
    price: Amount,
    amount: Option<Decimal>,
    fee: Amount,
    provider: &'a str,
    settlement: Option<NaiveDate>,
    tax_paid: Option<Amount>,
}

fn trade_values(record: &Stock) -> Option<TradeValues<'_>> {
    match &record.action {
        StockAction::Buy(buy) => Some(TradeValues {
            number: buy.number,
            price: buy.price,
            amount: buy.amount,
            fee: buy.fee,
            provider: &buy.provider,
            settlement: buy.settlement_date,
            tax_paid: None,
        }),
        StockAction::Sell(sell) => Some(TradeValues {
            number: sell.number,
            price: sell.price,
            amount: sell.amount,
            fee: sell.fee,
            provider: &sell.provider,
            settlement: sell.settlement_date,
            tax_paid: sell.tax_paid,
        }),
        StockAction::Split(_) => None,
    }
}

impl StockEditor {
    fn new(
        record: Option<&Stock>,
        prefill: Option<BuyPrefill>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let trade = record.and_then(trade_values);
        let split = record.and_then(|record| match &record.action {
            StockAction::Split(split) => Some((record.time, split.clone())),
            _ => None,
        });
        let action = match record.map(|record| &record.action) {
            Some(StockAction::Sell(_)) => Action::Sell,
            Some(StockAction::Split(_)) => Action::Split,
            Some(StockAction::Buy(_)) | None => Action::Buy,
        };
        // The missing buy came before the sale; at midnight it sorts before it too.
        let buy_time = prefill
            .as_ref()
            .and_then(|prefill| time::start_of_day(time::tax_date(prefill.first_sale)));
        let currency = trade
            .as_ref()
            .map(|trade| trade.price.currency)
            .or(prefill.as_ref().map(|prefill| prefill.currency))
            .unwrap_or(Currency::USD);
        let ticker = record
            .map(|record| record.ticker.clone())
            .or(prefill.as_ref().map(|prefill| prefill.ticker.clone()))
            .unwrap_or_default();
        let provider = trade
            .as_ref()
            .map(|trade| trade.provider.to_owned())
            .or(prefill.map(|prefill| prefill.provider))
            .unwrap_or_default();
        let text = |value: Option<String>| value.unwrap_or_default();
        let split_day = split.as_ref().map_or_else(
            || time::tax_date(Utc::now()),
            |(time, _)| time::tax_date(*time),
        );
        Self {
            existing: record.map(|record| record.id.clone()),
            action,
            id: form::optional_id(text(record.map(|record| record.id.clone())), window, cx),
            time: form::date_picker(
                record
                    .filter(|_| trade.is_some())
                    .map(|record| record.time)
                    .or(buy_time),
                window,
                cx,
            ),
            day: form::day_picker(Some(split_day), window, cx),
            ticker: form::input(ticker, window, cx),
            provider: form::input(provider, window, cx),
            exchange: exchange::picker_state(window, cx),
            exchange_picked: false,
            shares: form::input(
                text(trade.as_ref().map(|trade| trade.number.to_string())),
                window,
                cx,
            ),
            price: form::input(
                text(trade.as_ref().map(|trade| trade.price.value.to_string())),
                window,
                cx,
            ),
            price_currency: form::currency(currency, window, cx),
            kept_amount: trade.as_ref().and_then(|trade| {
                trade.amount.map(|amount| KeptAmount {
                    shares: trade.number,
                    price: (trade.price.value, trade.price.currency),
                    amount,
                })
            }),
            fee: form::input(
                trade
                    .as_ref()
                    .map_or_else(|| "0".into(), |trade| trade.fee.value.to_string()),
                window,
                cx,
            ),
            fee_currency: form::currency(
                trade.as_ref().map_or(currency, |trade| trade.fee.currency),
                window,
                cx,
            ),
            tax_paid: form::input(
                text(
                    trade
                        .as_ref()
                        .and_then(|trade| trade.tax_paid)
                        .map(|tax| tax.value.to_string()),
                ),
                window,
                cx,
            ),
            tax_currency: form::currency(
                trade
                    .as_ref()
                    .and_then(|trade| trade.tax_paid)
                    .map_or(currency, |tax| tax.currency),
                window,
                cx,
            ),
            settlement: form::day_picker(
                trade.as_ref().and_then(|trade| trade.settlement),
                window,
                cx,
            ),
            before: form::input(
                text(split.as_ref().map(|(_, split)| split.before.to_string())),
                window,
                cx,
            ),
            after: form::input(
                text(split.as_ref().map(|(_, split)| split.after.to_string())),
                window,
                cx,
            ),
            id_open: false,
            known: None,
            filled: None,
            draft: None,
            preview: None,
            preview_task: None,
            settles_key: None,
            settles: None,
            settles_task: None,
            known_task: None,
            _subscriptions: Vec::new(),
        }
    }

    fn title(&self) -> Cow<'static, str> {
        if self.existing.is_some() {
            t!("editor.edit")
        } else {
            t!("editor.add")
        }
    }

    fn is_trade(&self) -> bool {
        self.action != Action::Split
    }

    fn ticker(&self, cx: &App) -> String {
        form::text(&self.ticker, cx)
    }

    fn provider(&self, cx: &App) -> String {
        form::text(&self.provider, cx)
    }

    fn known_exchange(&self, cx: &App) -> Option<Mic> {
        let known = self.known.as_ref()?;
        known
            .listings
            .get(&self.ticker(cx), &self.provider(cx))
            .map(|listing| listing.mic)
    }

    // Saving the instrument's own exchange again could replace a segment MIC an import stored.
    fn exchange_change(&self, cx: &App) -> Option<Mic> {
        let picked = exchange::picked(&self.exchange, cx)?;
        (self.known_exchange(cx) != Some(picked)).then_some(picked)
    }

    fn amount(&self, cx: &App) -> Option<Decimal> {
        let kept = self.kept_amount?;
        let shares = form::parsed_decimal(&self.shares, cx)?;
        let price = form::parsed_amount(&self.price, &self.price_currency, cx)?;
        (shares == kept.shares && price == kept.price).then_some(kept.amount)
    }

    fn settles_key(&self, cx: &App) -> Option<SettlesKey> {
        if !self.is_trade() {
            return None;
        }
        Some(SettlesKey {
            time: form::picked_time(&self.time, cx)?,
            ticker: self.ticker(cx),
            provider: self.provider(cx),
            currency: self.price_currency.read(cx).selected_value().copied()?,
            exchange: exchange::picked(&self.exchange, cx),
        })
    }

    fn settles_shown(&self, cx: &App) -> Option<(NaiveDate, bool)> {
        match form::picked_day(&self.settlement, cx) {
            Some(day) => Some((day, false)),
            None => self.settles.map(|day| (day, true)),
        }
    }

    fn submission(&self, cx: &App) -> Result<StockSubmission, String> {
        let ticker = form::required(&self.ticker, &t!("field.ticker"), cx)?;
        let (time, action) = match self.action {
            Action::Split => {
                let day = form::picked_day(&self.day, cx)
                    .ok_or_else(|| t!("form.required", field = t!("field.date")).into_owned())?;
                let action = StockActionFields::Split {
                    before: form::required(&self.before, &t!("field.shares_before"), cx)?,
                    after: form::required(&self.after, &t!("field.shares_after"), cx)?,
                };
                (day.format("%Y-%m-%d 00:00:00").to_string(), action)
            }
            Action::Buy | Action::Sell => {
                let time = form::selected_time(&self.time, &t!("field.date_time"), cx)?;
                let trade = TradeFields {
                    shares: form::amount(&self.shares, &t!("field.shares"), cx)?,
                    price: form::amount(&self.price, &t!("field.price_per_share"), cx)?,
                    price_currency: form::selected(
                        &self.price_currency,
                        &t!("field.price_currency"),
                        cx,
                    )?,
                    amount: self.amount(cx),
                    fee: form::amount(&self.fee, &t!("field.fee"), cx)?,
                    fee_currency: form::selected(
                        &self.fee_currency,
                        &t!("field.fee_currency"),
                        cx,
                    )?,
                    provider: form::required(&self.provider, &t!("field.provider"), cx)?,
                    settlement_date: form::picked_day(&self.settlement, cx),
                    exchange: self.exchange_change(cx),
                };
                let action = if self.action == Action::Buy {
                    StockActionFields::Buy(trade)
                } else {
                    let tax_paid = match form::optional_amount(&self.tax_paid, cx) {
                        Some(value) => Some((
                            value,
                            form::selected(&self.tax_currency, &t!("field.tax_currency"), cx)?,
                        )),
                        None => None,
                    };
                    StockActionFields::Sell { trade, tax_paid }
                };
                (time, action)
            }
        };
        let fields = StockFields {
            time,
            ticker,
            action,
        };
        Ok(match self.existing.clone() {
            Some(id) => Submission::Update(UpdateStockInput { id, fields }),
            None => Submission::Create(CreateStockInput {
                id: form::optional(&self.id, cx),
                fields,
            }),
        })
    }

    fn draft(&self, cx: &App) -> Option<Draft> {
        let ticker = self.ticker(cx);
        if ticker.is_empty() {
            return None;
        }
        let (time, action, exchange) = match self.action {
            Action::Split => {
                let time = time::start_of_day(form::picked_day(&self.day, cx)?)?;
                let before = form::text(&self.before, cx).parse().ok()?;
                let after = form::text(&self.after, cx).parse().ok()?;
                (time, DraftAction::Split(before, after), None)
            }
            Action::Buy | Action::Sell => {
                let provider = self.provider(cx);
                if provider.is_empty() {
                    return None;
                }
                let trade = DraftTrade {
                    shares: form::parsed_decimal(&self.shares, cx)?,
                    price: form::parsed_amount(&self.price, &self.price_currency, cx)?,
                    amount: self.amount(cx),
                    fee: form::parsed_amount(&self.fee, &self.fee_currency, cx)?,
                    provider,
                    settlement: form::picked_day(&self.settlement, cx),
                };
                let action = if self.action == Action::Buy {
                    DraftAction::Buy(trade)
                } else {
                    let tax_paid = if form::text(&self.tax_paid, cx).is_empty() {
                        None
                    } else {
                        Some(form::parsed_amount(&self.tax_paid, &self.tax_currency, cx)?)
                    };
                    DraftAction::Sell(trade, tax_paid)
                };
                let time = form::picked_time(&self.time, cx)?;
                (time, action, exchange::picked(&self.exchange, cx))
            }
        };
        Some(Draft {
            // Stored IDs are never empty, so a new record replaces none in the preview.
            id: self.existing.clone().unwrap_or_default(),
            time,
            ticker,
            action,
            exchange,
        })
    }

    fn ticker_hint(&self, cx: &App) -> Option<String> {
        let known = self.known.as_ref()?;
        let ticker = self.ticker(cx);
        if ticker.is_empty() {
            return None;
        }
        let held = known
            .instruments
            .iter()
            .filter(|instrument| instrument.ticker == ticker)
            .collect::<Vec<_>>();
        if held.is_empty() {
            return Some(t!("editor.new_ticker").into_owned());
        }
        let exchange = |provider: &str| {
            known
                .listings
                .get(&ticker, provider)
                .map(|listing| listing.name.clone())
        };
        let providers = held
            .iter()
            .map(|instrument| instrument.provider.as_str())
            .collect::<Vec<_>>();
        if !self.is_trade() {
            let mut exchanges = held
                .iter()
                .map(|instrument| exchange(&instrument.provider))
                .collect::<Vec<_>>();
            exchanges.sort();
            exchanges.dedup();
            let held_in = and_list(&providers);
            return Some(
                match exchanges.as_slice() {
                    [Some(name)] => t!("editor.held_on", exchange = name, providers = held_in),
                    _ => t!("editor.held", providers = held_in),
                }
                .into_owned(),
            );
        }
        let provider = self.provider(cx);
        Some(
            match held
                .iter()
                .find(|instrument| instrument.provider == provider)
            {
                Some(_) => match exchange(&provider) {
                    Some(name) => t!("editor.held_on", exchange = name, providers = provider),
                    None => t!("editor.held_no_exchange", provider = provider),
                },
                None if provider.is_empty() => {
                    t!("editor.held", providers = and_list(&providers))
                }
                None => t!(
                    "editor.held_new_in",
                    providers = and_list(&providers),
                    provider = provider
                ),
            }
            .into_owned(),
        )
    }

    fn exchange_hint(&self, catalog: &Catalog, cx: &App) -> Result<String, String> {
        if let Some(error) = &catalog.refresh_error {
            return Err(error.to_string());
        }
        let ticker = self.ticker(cx);
        let ticker = if ticker.is_empty() {
            t!("editor.the_ticker").into_owned()
        } else {
            ticker
        };
        Ok(t!("editor.exchange_hint", ticker = ticker).into_owned())
    }

    fn time_hint(&self, cx: &App) -> String {
        match self.settles_shown(cx) {
            Some((day, true)) => t!("editor.settles_worked_out", date = date(day).main),
            Some((day, false)) => t!("editor.settles", date = date(day).main),
            None => t!("editor.warsaw_time"),
        }
        .into_owned()
    }

    fn split_hint(&self, cx: &App) -> String {
        let before = form::text(&self.before, cx).parse::<u32>();
        let after = form::text(&self.after, cx).parse::<u32>();
        match (before, after) {
            (Ok(before), Ok(after)) if before > 0 && after > 0 => {
                t!("editor.split_hint", ratio = ratio(before, after)).into_owned()
            }
            _ => t!("editor.split_hint_default").into_owned(),
        }
    }
}

#[derive(Clone, PartialEq)]
struct Draft {
    id: String,
    time: DateTime<Utc>,
    ticker: String,
    action: DraftAction,
    exchange: Option<Mic>,
}

#[derive(Clone, PartialEq)]
enum DraftAction {
    Buy(DraftTrade),
    Sell(DraftTrade, Option<(Decimal, Currency)>),
    Split(u32, u32),
}

#[derive(Clone, PartialEq)]
struct DraftTrade {
    shares: Decimal,
    price: (Decimal, Currency),
    amount: Option<Decimal>,
    fee: (Decimal, Currency),
    provider: String,
    settlement: Option<NaiveDate>,
}

fn to_amount((value, currency): (Decimal, Currency)) -> Amount {
    Amount { value, currency }
}

impl Draft {
    fn stock(&self) -> Stock {
        let trade = |trade: &DraftTrade| StockBuy {
            number: trade.shares,
            price: to_amount(trade.price),
            amount: trade.amount,
            fee: to_amount(trade.fee),
            provider: trade.provider.clone(),
            settlement_date: trade.settlement,
        };
        let action = match &self.action {
            DraftAction::Buy(buy) => StockAction::Buy(trade(buy)),
            DraftAction::Sell(sell, tax_paid) => {
                let StockBuy {
                    number,
                    price,
                    amount,
                    fee,
                    provider,
                    settlement_date,
                } = trade(sell);
                StockAction::Sell(StockSell {
                    number,
                    price,
                    amount,
                    fee,
                    provider,
                    settlement_date,
                    tax_paid: tax_paid.map(to_amount),
                })
            }
            DraftAction::Split(before, after) => StockAction::Split(StockSplit {
                before: *before,
                after: *after,
            }),
        };
        Stock {
            id: self.id.clone(),
            time: self.time,
            ticker: self.ticker.clone(),
            action,
        }
    }
}

enum RateNote {
    Nbp(NaiveDate),
    Pln,
    None,
}

struct PreviewLine {
    lead: Option<SharedString>,
    text: SharedString,
}

struct PreviewResult {
    label: SharedString,
    value: DisplayText,
    note: Option<SharedString>,
}

struct PreviewView {
    // A sale's own; a buy shows the editor's hint.
    settles: Option<NaiveDate>,
    rate: RateNote,
    formula: Vec<DisplayText>,
    /// The unit is a message key.
    value: Option<(DisplayText, &'static str)>,
    lines: Vec<PreviewLine>,
    results: Vec<PreviewResult>,
    caption: Option<SharedString>,
}

fn line(text: impl Into<SharedString>) -> PreviewLine {
    PreviewLine {
        lead: None,
        text: text.into(),
    }
}

fn result(
    label: impl Into<SharedString>,
    value: DisplayText,
    note: Option<String>,
) -> PreviewResult {
    PreviewResult {
        label: label.into(),
        value,
        note: note.map(Into::into),
    }
}

fn rate_note(currency: Currency, nbp_date: NaiveDate) -> RateNote {
    if currency == Currency::PLN {
        RateNote::Pln
    } else {
        RateNote::Nbp(nbp_date)
    }
}

/// Every part, unlike `i18n::and_list`, which shortens long lists.
fn join_and(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => t!("list.and", items = rest.join(", "), last = last).into_owned(),
    }
}

fn no_buy(shares: Decimal) -> String {
    let key = plural_key_decimal("editor.no_buy", shares);
    t!(key, shares = shares_count(shares)).into_owned()
}

fn fifo_line(sale: &CalculatedSale) -> String {
    let taken = sale
        .matches
        .iter()
        .map(|lot| {
            let (taken, date) = (shares_text(lot.taken), trade_date(lot.bought));
            if lot.after.is_zero() {
                t!("editor.from_lot_closes", taken = taken, date = date)
            } else {
                t!("editor.from_lot", taken = taken, date = date)
            }
            .into_owned()
        })
        .collect::<Vec<_>>();
    let uncovered = sale.uncovered > Decimal::ZERO;
    let provider = &sale.provider;
    match (taken.is_empty(), uncovered) {
        (true, _) => t!(
            "editor.fifo_none",
            provider = provider,
            no_buy = no_buy(sale.uncovered)
        ),
        (false, false) => t!(
            "editor.fifo_takes",
            provider = provider,
            taken = join_and(&taken)
        ),
        (false, true) => t!(
            "editor.fifo_takes_no_buy",
            provider = provider,
            taken = join_and(&taken),
            no_buy = no_buy(sale.uncovered)
        ),
    }
    .into_owned()
}

impl PreviewView {
    fn new(preview: &StockPreview, draft: &Draft) -> Self {
        match preview {
            StockPreview::Sell { sale, holding } => {
                let result_label = if sale.profit < Decimal::ZERO {
                    t!("home.loss")
                } else {
                    t!("home.profit")
                };
                let tax_note = if sale.country.is_some() {
                    "→ PIT/ZG 30".to_owned()
                } else {
                    t!("editor.no_pit_zg").into_owned()
                };
                Self {
                    settles: Some(sale.settlement_date),
                    rate: rate_note(sale.price.currency, sale.nbp_date),
                    formula: conversion(
                        sale.shares,
                        sale.price,
                        sale.from_total
                            .then_some((sale.value, sale.provider.as_str())),
                        sale.nbp_rate,
                    ),
                    value: Some((pln(sale.income), "editor.unit_income")),
                    lines: vec![line(fifo_line(sale))],
                    results: vec![
                        result(t!("column.costs"), pln(sale.costs), None),
                        result(result_label, pln(sale.profit.abs()), None),
                        result(
                            t!("editor.left_after"),
                            DisplayText::plain(shares_count(holding.shares)),
                            None,
                        ),
                        result(
                            t!("editor.tax_abroad"),
                            pln(sale.tax_paid_pln),
                            Some(tax_note),
                        ),
                    ],
                    caption: sale.from_total.then(|| {
                        format!("{}.", total_note(&sale.provider, sale.value.currency)).into()
                    }),
                }
            }
            StockPreview::Buy { lot, holding } => {
                let fee_note = match &draft.action {
                    DraftAction::Buy(trade) if trade.fee.1 == Currency::PLN => {
                        Some(t!("editor.fee_no_conversion").into_owned())
                    }
                    DraftAction::Buy(trade) if !trade.fee.0.is_zero() => {
                        let fee = to_amount(trade.fee);
                        Some(format!(
                            "· {} × {}",
                            price_text(fee),
                            fee_rate(fee, lot.lot_fee, lot.price, lot.nbp_rate)
                        ))
                    }
                    _ => None,
                };
                Self {
                    settles: None,
                    rate: rate_note(lot.price.currency, lot.nbp_date),
                    formula: conversion(
                        lot.lot_shares,
                        lot.price,
                        lot.from_total
                            .then_some((lot.lot_value, lot.provider.as_str())),
                        lot.nbp_rate,
                    ),
                    value: Some((pln(lot.lot_cost), "editor.unit_lot_cost")),
                    lines: vec![line(t!("editor.new_lot", provider = lot.provider))],
                    results: vec![
                        result(t!("field.fee"), pln(lot.lot_fee), fee_note),
                        result(
                            t!("editor.held_after", ticker = lot.ticker),
                            DisplayText::plain(shares_in_lots(holding.shares, holding.lots)),
                            None,
                        ),
                    ],
                    caption: lot.from_total.then(|| {
                        format!("{}.", total_note(&lot.provider, lot.lot_value.currency)).into()
                    }),
                }
            }
            StockPreview::Split { lots, .. } => {
                let mut changed = lots
                    .iter()
                    .filter_map(|lot| {
                        lot.splits
                            .iter()
                            .find(|step| step.split_id == draft.id)
                            .map(|step| (lot, step))
                    })
                    .collect::<Vec<_>>();
                changed.sort_by(|(a, _), (b, _)| {
                    (&a.provider, a.bought).cmp(&(&b.provider, b.bought))
                });
                let ticker = &draft.ticker;
                let lines = if changed.is_empty() {
                    vec![line(t!("editor.no_open_lots", ticker = ticker))]
                } else {
                    let mut lines = vec![line(t!("editor.split_changes", ticker = ticker))];
                    lines.extend(changed.iter().map(|(lot, step)| {
                        PreviewLine {
                            lead: Some(
                                t!(
                                    "sale.provider_lot",
                                    provider = lot.provider,
                                    date = trade_date(lot.bought)
                                )
                                .into(),
                            ),
                            text: t!(
                                "sale.split_change",
                                before = shares_text(step.lot_shares_before),
                                price_before = money(step.price_before.value).full,
                                after = shares_text(step.lot_shares_after),
                                price_after = price_text(step.price_after)
                            )
                            .into(),
                        }
                    }));
                    lines
                };
                Self {
                    settles: None,
                    rate: RateNote::None,
                    formula: Vec::new(),
                    value: None,
                    lines,
                    results: Vec::new(),
                    caption: (!changed.is_empty()).then(|| t!("editor.split_costs_note").into()),
                }
            }
        }
    }
}

impl StocksPage {
    pub fn open_editor(
        &mut self,
        record: Option<&Stock>,
        prefill: Option<BuyPrefill>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled() {
            return;
        }
        let prefilled = prefill.is_some();
        let mut editor = StockEditor::new(record, prefill, window, cx);
        let focus = if prefilled {
            editor.shares.focus_handle(cx)
        } else {
            editor.ticker.focus_handle(cx)
        };
        let changed = Self::editor_changed;
        let mut subscriptions = vec![
            form::watch(&editor.time, window, cx, changed),
            form::watch(&editor.day, window, cx, changed),
            form::watch(&editor.ticker, window, cx, changed),
            form::watch(&editor.provider, window, cx, changed),
            form::watch(&editor.exchange, window, cx, changed),
            form::watch(&editor.shares, window, cx, changed),
            form::watch(&editor.price, window, cx, changed),
            form::watch(&editor.price_currency, window, cx, changed),
            form::watch(&editor.fee, window, cx, changed),
            form::watch(&editor.fee_currency, window, cx, changed),
            form::watch(&editor.tax_paid, window, cx, changed),
            form::watch(&editor.tax_currency, window, cx, changed),
            form::watch(&editor.settlement, window, cx, changed),
            form::watch(&editor.before, window, cx, changed),
            form::watch(&editor.after, window, cx, changed),
        ];
        subscriptions.push(cx.subscribe_in(
            &editor.exchange,
            window,
            |this, _, event: &SelectEvent<ExchangeList>, window, cx| {
                let SelectEvent::Confirm(_) = event;
                if let Some(editor) = &mut this.editor {
                    editor.exchange_picked = true;
                }
                this.editor_changed(window, cx);
            },
        ));
        subscriptions.push(cx.observe_in(&self.catalog, window, |this, _, window, cx| {
            this.fill_editor_picker(window, cx);
            this.editor_changed(window, cx);
        }));
        editor._subscriptions = subscriptions;
        self.editor = Some(editor);
        self.status.error = None;
        self.status.message = None;
        dialog::open(window, cx, Self::render_editor);
        window.focus(&focus, cx);
        self.load_known(window, cx);
        self.fill_editor_picker(window, cx);
        self.catalog
            .update(cx, |catalog, cx| catalog.load(window, cx));
        self.editor_changed(window, cx);
        self.notify(cx);
    }

    fn editor_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        // Follows the instrument until the user picks one, and only to a MIC the list has, so
        // setting it doesn't fail and notify again.
        if !editor.exchange_picked && editor.is_trade() && editor.known.is_some() {
            let catalog = self.catalog.read(cx);
            let mic = editor
                .known_exchange(cx)
                .filter(|mic| catalog.contains(*mic));
            exchange::select(&editor.exchange, mic, window, cx);
        }
        self.update_settles(window, cx);
        self.update_stock_preview(window, cx);
        cx.notify();
    }

    fn update_settles(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let key = editor.settles_key(cx);
        if key == editor.settles_key {
            return;
        }
        let task = key.clone().map(|key| {
            self.context.services.run(
                window,
                cx,
                move |app| async move {
                    Ok(stock::settlement_hint(
                        &app,
                        key.time,
                        &key.ticker,
                        &key.provider,
                        key.currency,
                        key.exchange,
                    )
                    .await?)
                },
                |this, result, _, cx| {
                    if let Some(editor) = &mut this.editor {
                        editor.settles = result.ok();
                        cx.notify();
                    }
                },
            )
        });
        if let Some(editor) = &mut self.editor {
            if key.is_none() {
                editor.settles = None;
            }
            editor.settles_key = key;
            editor.settles_task = task;
        }
    }

    fn update_stock_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let draft = editor.draft(cx);
        if draft == editor.draft {
            return;
        }
        let task = draft.clone().map(|draft| {
            self.context.services.run(
                window,
                cx,
                move |app| async move {
                    let preview = stock::preview_stock(&app, draft.stock(), draft.exchange).await?;
                    Ok(PreviewView::new(&preview, &draft))
                },
                |this, result, _, cx| {
                    if let Some(editor) = &mut this.editor {
                        editor.preview = Some(result.map_err(SharedString::from));
                        cx.notify();
                    }
                },
            )
        });
        if let Some(editor) = &mut self.editor {
            if draft.is_none() {
                editor.preview = None;
            }
            editor.draft = draft;
            editor.preview_task = task;
        }
    }

    fn fill_editor_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        let catalog = self.catalog.read(cx);
        let (empty, revision) = (catalog.is_empty(), catalog.revision);
        if empty || editor.filled == Some(revision) {
            return;
        }
        editor.filled = Some(revision);
        exchange::fill(&editor.exchange, &self.catalog, window, cx);
    }

    fn load_known(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let task = self.context.services.run(
            window,
            cx,
            |app| async move {
                let instruments = stock::list_instruments(&app).await?;
                let listings = stock::load_listings(&app).await?;
                Ok((instruments, listings))
            },
            |this, result, window, cx| {
                let Some(editor) = &mut this.editor else {
                    return;
                };
                match result {
                    Ok((instruments, listings)) => {
                        editor.known = Some(Known {
                            instruments,
                            listings,
                        });
                    }
                    Err(error) => this.status.error = Some(error.into()),
                }
                this.editor_changed(window, cx);
            },
        );
        if let Some(editor) = &mut self.editor {
            editor.known_task = Some(task);
        }
    }

    fn pick_action(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        if editor.existing.is_some() || editor.action == action {
            return;
        }
        editor.action = action;
        self.editor_changed(window, cx);
    }

    fn toggle_editor_id(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        editor.id_open = !editor.id_open;
        if editor.id_open && editor.existing.is_none() {
            let id = editor.id.focus_handle(cx);
            window.focus(&id, cx);
        }
        cx.notify();
    }

    fn close_editor(&mut self, cx: &mut Context<Self>) {
        self.editor = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn save_stock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(editor) = &self.editor else {
            return;
        };
        let submission = match editor.submission(cx) {
            Ok(submission) => submission,
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
                return;
            }
        };
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                match submission {
                    Submission::Create(input) => stock::create_stock(&app, input).await.map(drop),
                    Submission::Update(input) => stock::update_stock(&app, input).await,
                }?;
                Ok(t!("records.saved").into_owned())
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    this.context.years_changed(cx);
                    window.close_dialog(cx);
                    this.close_editor(cx);
                    window.focus(&this.focus, cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn render_editor(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(editor) = &self.editor else {
            return dialog;
        };
        let busy = self.status.busy;
        let p = *palette(cx);
        let fields = if editor.is_trade() {
            self.trade_fields(editor, busy, cx)
        } else {
            Self::split_fields(editor, busy, cx)
        };
        let body = v_flex()
            .pt(px(8.))
            .child(self.action_switch(editor, busy, cx))
            .child(fields.mt(px(14.)))
            .child(self.id_field(editor, busy, cx).mt(px(14.)))
            .when_some(self.status.error.as_ref(), |body, error| {
                body.child(form::error(error.text(), cx).mt(px(12.)))
            })
            .when_some(editor.preview.as_ref(), |body, preview| {
                body.child(Self::preview_band(editor, preview, cx).mt(px(16.)))
            });
        dialog::editor(
            self,
            dialog,
            body,
            if busy {
                t!("common.saving")
            } else {
                t!("common.save")
            },
            |this| &this.status,
            Self::save_stock,
            Self::close_editor,
            cx,
        )
        .title(div().text_size(px(15.)).child(editor.title()))
        .w(px(580.))
        .bg(p.popover)
        .border_color(p.strong_line)
        .px(px(20.))
        .pb(px(14.))
    }

    fn action_switch(&self, editor: &StockEditor, busy: bool, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let locked = editor.existing.is_some();
        h_flex().child(
            h_flex()
                .id("stock-action")
                .role(Role::RadioGroup)
                .aria_label(t!("field.action"))
                .gap(px(2.))
                .p(px(2.))
                .rounded(px(9.))
                .bg(p.selected)
                .children([Action::Buy, Action::Sell, Action::Split].map(|action| {
                    let selected = editor.action == action;
                    let title = action.title();
                    Button::new(("stock-action", action as usize))
                        .ghost()
                        .role(Role::RadioButton)
                        .h(px(26.))
                        .px(px(16.))
                        .rounded(px(7.))
                        .accessibility_label(if selected {
                            t!("editor.selected", action = title)
                        } else {
                            title.clone()
                        })
                        .disabled(busy || locked && !selected)
                        .when(selected, |button| {
                            button.bg(cx.theme().tokens.button).shadow_xs()
                        })
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_medium()
                                .text_color(if selected { p.text } else { p.muted })
                                .child(title),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.pick_action(action, window, cx)
                        }))
                })),
        )
    }

    fn trade_fields(&self, editor: &StockEditor, busy: bool, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let exchange_hint = match editor.exchange_hint(self.catalog.read(cx), cx) {
            Ok(lead) => form::hint_ending(lead, "PIT/ZG", cx),
            Err(error) => form::hint(error, cx).text_color(p.warning),
        };
        let exchange = form::field(
            t!("field.exchange"),
            div().w_full().h_8().flex_shrink_0().child(exchange::picker(
                &editor.exchange,
                &self.catalog,
                t!("field.exchange"),
                busy,
                cx,
            )),
            cx,
        )
        .child(exchange_hint);
        let settlement = form::field(
            form::noted_label(t!("field.settlement_date"), t!("form.optional"), cx),
            DatePicker::new(&editor.settlement)
                .cleanable(true)
                .placeholder(t!("editor.settlement_placeholder"))
                .disabled(busy),
            cx,
        );
        form::grid()
            .child(
                form::field(
                    t!("field.date_time"),
                    DatePicker::new(&editor.time).disabled(busy),
                    cx,
                )
                .child(form::hint(editor.time_hint(cx), cx)),
            )
            .child(
                form::input_field(t!("field.ticker"), &editor.ticker, busy, cx)
                    .children(editor.ticker_hint(cx).map(|hint| form::hint(hint, cx))),
            )
            .child(exchange)
            .child(form::input_field(
                t!("field.provider"),
                &editor.provider,
                busy,
                cx,
            ))
            .child(form::input_field(
                t!("field.shares"),
                &editor.shares,
                busy,
                cx,
            ))
            .child(form::amount_field(
                t!("field.price_per_share"),
                t!("field.price_currency"),
                &editor.price,
                &editor.price_currency,
                busy,
                cx,
            ))
            .child(form::amount_field(
                t!("field.fee"),
                t!("field.fee_currency"),
                &editor.fee,
                &editor.fee_currency,
                busy,
                cx,
            ))
            .when(editor.action == Action::Sell, |grid| {
                grid.child(form::field(
                    form::noted_label(t!("field.tax_withheld"), t!("editor.tax_withheld_note"), cx),
                    form::amount_control(
                        t!("field.tax_withheld"),
                        t!("field.tax_currency"),
                        &editor.tax_paid,
                        &editor.tax_currency,
                        busy,
                        cx,
                    ),
                    cx,
                ))
            })
            .child(settlement)
    }

    fn split_fields(editor: &StockEditor, busy: bool, cx: &App) -> Div {
        form::grid()
            .child(
                form::field(
                    t!("field.date"),
                    DatePicker::new(&editor.day).disabled(busy),
                    cx,
                )
                .child(form::hint(t!("editor.split_day_hint"), cx)),
            )
            .child(
                form::input_field(t!("field.ticker"), &editor.ticker, busy, cx)
                    .children(editor.ticker_hint(cx).map(|hint| form::hint(hint, cx))),
            )
            .child(form::input_field(
                t!("field.shares_before"),
                &editor.before,
                busy,
                cx,
            ))
            .child(
                form::input_field(t!("field.shares_after"), &editor.after, busy, cx)
                    .child(form::hint(editor.split_hint(cx), cx)),
            )
    }

    fn id_field(&self, editor: &StockEditor, busy: bool, cx: &mut Context<Self>) -> Collapsible {
        let p = *palette(cx);
        let note = match &editor.existing {
            Some(id) => format!("· {id}"),
            None => t!("records.id_note").into_owned(),
        };
        Collapsible::new()
            .open(editor.id_open)
            .gap(px(10.))
            .child(
                h_flex().child(
                    Button::new("stock-editor-id")
                        .ghost()
                        .h(px(22.))
                        .px(px(4.))
                        .ml(px(-4.))
                        .gap(px(6.))
                        .rounded(px(6.))
                        .text_color(p.muted)
                        .accessibility_label(t!("field.id"))
                        .child(
                            Icon::new(if editor.id_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .size(px(12.)),
                        )
                        .child(
                            h_flex()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .child(t!("field.id"))
                                .child(div().text_color(p.faint).child(note)),
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.toggle_editor_id(window, cx)),
                        ),
                ),
            )
            .content(form::input_field(
                t!("field.id"),
                &editor.id,
                busy || editor.existing.is_some(),
                cx,
            ))
    }

    fn preview_band(
        editor: &StockEditor,
        preview: &Result<PreviewView, SharedString>,
        cx: &App,
    ) -> Div {
        let p = *palette(cx);
        let preview = match preview {
            Ok(preview) => preview,
            Err(error) => return preview_error(error.clone(), cx),
        };
        let settles = preview
            .settles
            .or_else(|| editor.settles_shown(cx).map(|(day, _)| day))
            .map(|day| t!("editor.preview_settles", date = date(day).main));
        let rate = match preview.rate {
            RateNote::Nbp(day) => Some(t!("records.nbp_date", date = date(day).main)),
            RateNote::Pln => Some(t!("sale.no_conversion")),
            RateNote::None => None,
        };
        let note = match (settles, rate) {
            (Some(settles), Some(rate)) => format!("{settles} · {rate}"),
            (None, Some(rate)) => rate.into_owned(),
            (_, None) => t!("editor.no_rate").into_owned(),
        };
        preview_frame(Some(note.into()), cx)
            .when_some(preview.value.as_ref(), |band, (value, unit)| {
                band.child(
                    h_flex()
                        .flex_wrap()
                        .gap_x(px(4.))
                        .text_color(p.muted)
                        .children(preview.formula.iter().enumerate().map(|(index, part)| {
                            preview_value(("stock-preview-formula", index), part, div(), cx)
                        }))
                        .child("=")
                        .child(preview_value(
                            "stock-preview-value",
                            value,
                            div().font_semibold().text_color(p.text),
                            cx,
                        ))
                        .child(div().text_color(p.faint).child(t!(*unit))),
                )
            })
            .children(preview.lines.iter().map(|line| {
                h_flex()
                    .flex_wrap()
                    .items_baseline()
                    .gap_x(px(8.))
                    .text_color(p.muted)
                    .when_some(line.lead.clone(), |row, lead| {
                        row.child(div().text_color(p.text).child(lead))
                    })
                    .child(div().min_w_0().child(line.text.clone()))
            }))
            .when(!preview.results.is_empty(), |band| {
                band.child(
                    h_flex()
                        .flex_wrap()
                        .gap_x(px(20.))
                        .gap_y(px(4.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .children(preview.results.iter().enumerate().map(|(index, result)| {
                            h_flex()
                                .gap(px(4.))
                                .child(result.label.clone())
                                .child(preview_value(
                                    ("stock-preview-result", index),
                                    &result.value,
                                    div().text_color(p.text),
                                    cx,
                                ))
                                .children(result.note.clone())
                        })),
                )
            })
            .when_some(preview.caption.clone(), |band, caption| {
                band.child(div().text_size(px(12.)).text_color(p.muted).child(caption))
            })
    }
}
