mod display;
mod editor;
mod exchange;
mod history;
mod holdings;
mod portfolio;
mod realized;
mod set_exchange;

use crate::{
    components::{
        ButtonText, Status, dialog, header, nbp, notice,
        records::{CellStyle, RecordColumn, TableLayout},
        tabs::{self, TabItem},
        value,
    },
    format::{DisplayText, pln, record_count},
    i18n::{or_list, plural, plural_key},
    navigation::{Page, PageContext},
    pages::records::{
        RecordTable, TableHost, delete_selected_message, filter_count, filter_field, import_button,
        no_match, selection_bar, table, totals_row,
    },
    theme::palette,
};
use chrono::{Datelike, Utc};
use display::{Action, InstrumentKey, StockDisplay, WarningAction, WarningCard};
use editor::StockEditor;
use exchange::Catalog;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        button::*,
        dialog::Dialog,
        empty::{Empty, EmptyContent},
        input::{InputEvent, InputState},
        scroll::ScrollableElement,
        *,
    },
    *,
};
use history::{HistoryEvent, HistoryView};
use pitpls_app::use_case::{
    portfolio as portfolio_use_case, stock,
    year::{self, YearInfo},
};
use pitpls_core::time::tax_date;
use pitpls_importers::{IMPORTERS, OutputType};
use rust_decimal::Decimal;
use rust_i18n::t;
use set_exchange::{ExchangeDialog, ExchangeDialogEvent, ExchangeRow};
use std::collections::HashSet;

const NARROW: f32 = 600.;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Sales,
    Realized,
    Records,
    Holdings,
}

#[derive(Clone, Copy)]
pub enum Table {
    Sales,
    Records,
}

pub struct StocksPage {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    tab: Tab,
    display: Option<StockDisplay>,
    sales: RecordTable,
    records: RecordTable,
    holdings: holdings::Layout,
    portfolio: portfolio::Layout,
    realized: realized::Layout,
    filter: Entity<InputState>,
    query: String,
    page_scroll: ScrollHandle,
    editor: Option<StockEditor>,
    catalog: Entity<Catalog>,
    exchange_dialog: Option<(Entity<ExchangeDialog>, Subscription)>,
    pending_delete: Option<Vec<String>>,
    deleting: HashSet<String>,
    nbp_year: Option<Entity<InputState>>,
    missing_rate: Option<SharedString>,
    missing_rate_year: Option<i32>,
    other_year: Option<(i32, u32)>,
    history: Option<(Entity<HistoryView>, Subscription)>,
    focus: FocusHandle,
    _subscriptions: [Subscription; 2],
}

fn sale_columns() -> Vec<RecordColumn> {
    vec![
        RecordColumn::new("column.date", CellStyle::Time, 96., 50.),
        RecordColumn::new("column.ticker", CellStyle::Strong, 64., 56.),
        RecordColumn::new("column.exchange", CellStyle::Muted, 80., 64.),
        RecordColumn::grow("column.provider", CellStyle::Muted),
        RecordColumn::new("column.shares", CellStyle::Number, 64., 56.),
        RecordColumn::new("column.income", CellStyle::Number, 130., 110.),
        RecordColumn::new("column.costs", CellStyle::Number, 130., 110.).optional(),
        RecordColumn::new("column.profit", CellStyle::Number, 130., 110.),
    ]
}

fn record_columns(all_years: bool) -> Vec<RecordColumn> {
    vec![
        RecordColumn::new("column.date", CellStyle::Time, 96., 50.),
        RecordColumn::new(
            "column.action",
            CellStyle::Tag,
            if all_years { 80. } else { 64. },
            if all_years { 80. } else { 56. },
        ),
        RecordColumn::new("column.ticker", CellStyle::Strong, 64., 56.),
        RecordColumn::new("column.exchange", CellStyle::Muted, 80., 64.).optional(),
        RecordColumn::grow("column.provider", CellStyle::Muted),
        RecordColumn::new("column.shares", CellStyle::Number, 64., 56.),
        RecordColumn::new("column.price", CellStyle::Number, 104., 90.).optional(),
        RecordColumn::new("column.value", CellStyle::Number, 120., 100.),
        RecordColumn::new("column.fee", CellStyle::Number, 90., 80.).optional(),
    ]
}

impl StocksPage {
    pub fn new(
        context: PageContext,
        year: Option<i32>,
        width: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(t!("records.filter")));
        let filter_subscription = cx.subscribe_in(&filter, window, |this, filter, event, _, cx| {
            if let InputEvent::Change = event {
                let query = filter.read(cx).value().trim().to_lowercase();
                if query != this.query {
                    this.query = query;
                    this.regroup();
                    this.sales.state.scroll_to_top();
                    this.records.state.scroll_to_top();
                    cx.notify();
                }
            }
        });
        // As on the record pages: focus that falls back to the page or above
        // it stays on the page.
        let focus_subscription = cx.on_focus_lost(window, |this, window, cx| {
            let fallback = window.focus_lost_restore_target(cx);
            if !window.has_active_dialog(cx)
                && fallback.is_none_or(|target| {
                    target == this.focus || target.contains(&this.focus, window)
                })
            {
                window.focus(&this.focus, cx);
            }
        });
        let catalog = cx.new(|_| Catalog::new(context.services.clone()));
        let mut view = Self {
            context,
            year,
            status: Status::default(),
            tab: if year.is_some() {
                Tab::Sales
            } else {
                Tab::Holdings
            },
            display: None,
            sales: RecordTable::new("stock-sales", sale_columns(), false, width),
            records: RecordTable::new("stock-records", record_columns(year.is_none()), true, width),
            holdings: holdings::Layout::new(width),
            portfolio: portfolio::Layout::default(),
            realized: realized::Layout::new(width),
            filter,
            query: String::new(),
            page_scroll: ScrollHandle::default(),
            editor: None,
            catalog,
            exchange_dialog: None,
            pending_delete: None,
            deleting: HashSet::new(),
            nbp_year: None,
            missing_rate: None,
            missing_rate_year: None,
            other_year: None,
            history: None,
            focus: cx.focus_handle(),
            _subscriptions: [filter_subscription, focus_subscription],
        };
        view.refresh(window, cx);
        view
    }

    fn is_empty(&self) -> bool {
        self.display.as_ref().is_none_or(|display| {
            if self.year.is_none() {
                return display.stocks.is_empty();
            }
            self.sales.state.rows.is_empty()
                && self.records.state.rows.is_empty()
                && display.holdings.is_empty()
        })
    }

    fn locked(&self) -> bool {
        self.status.busy
            || self.editor.is_some()
            || self.exchange_dialog.is_some()
            || self.pending_delete.is_some()
            || self.nbp_year.is_some()
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(self.locked(), cx);
        cx.notify();
    }

    fn width(&self) -> Option<Pixels> {
        match self.tab {
            Tab::Sales => self.sales.state.viewport_width,
            Tab::Realized => self.realized.width,
            Tab::Records => self.records.state.viewport_width,
            Tab::Holdings => self.holdings.width,
        }
    }

    /// The tab coming in starts at the width the last one measured.
    fn select_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if tab == self.tab {
            return;
        }
        let width = self.width();
        self.tab = tab;
        match tab {
            Tab::Sales => self.sales.state.viewport_width = width,
            Tab::Realized => self.realized.width = width,
            Tab::Records => self.records.state.viewport_width = width,
            Tab::Holdings => self.holdings.width = width,
        }
        cx.notify();
    }

    fn open_history(&mut self, key: InstrumentKey, window: &mut Window, cx: &mut Context<Self>) {
        let year = self.year;
        if self.disabled() {
            return;
        }
        let (context, catalog, width) = (self.context.clone(), self.catalog.clone(), self.width());
        let view = cx.new(|cx| HistoryView::new(context, key, year, catalog, width, window, cx));
        let subscription =
            cx.subscribe_in(&view, window, |this, _, event, window, cx| match event {
                HistoryEvent::Back => {
                    this.history = None;
                    window.focus(&this.focus, cx);
                    cx.notify();
                }
                // The page behind shows the instrument's exchange too.
                HistoryEvent::Changed => this.refresh(window, cx),
            });
        let focus = view.read(cx).focus().clone();
        window.focus(&focus, cx);
        self.history = Some((view, subscription));
        cx.notify();
    }

    fn apply(&mut self, display: Option<StockDisplay>, other_year: Option<(i32, u32)>) {
        self.other_year = other_year;
        let Some(mut display) = display else {
            self.display = None;
            return;
        };
        let sales = self.sales.set_rows(std::mem::take(&mut display.sales));
        let records = self.records.set_rows(std::mem::take(&mut display.records));
        self.holdings.fit = None;
        self.display = Some(display);
        if sales || records {
            self.regroup();
        }
    }

    fn regroup(&mut self) {
        let Some(display) = &self.display else {
            return;
        };
        let mut sales = self.sales.month_groups(&self.query, false);
        for group in &mut sales {
            group.count = plural("stocks.sales_count", group.records.len() as u64).into();
            let profit = group
                .records
                .iter()
                .map(|&index| display.profits[index])
                .sum::<Decimal>();
            group.summary = vec![(Some("records.subtotal_profit"), pln(profit))];
        }
        self.sales.set_groups(sales);

        let mut records = self.records.month_groups(&self.query, self.year.is_none());
        for group in &mut records {
            group.count = record_count(group.records.len() as u32).into();
            let count = |action| {
                group
                    .records
                    .iter()
                    .filter(|&&index| display.actions[index] == action)
                    .count()
            };
            let parts = [
                (count(Action::Buy), "count.buys"),
                (count(Action::Sell), "stocks.sales_count"),
                (count(Action::Split), "count.splits"),
            ]
            .into_iter()
            .filter(|(count, _)| *count > 0)
            .map(|(count, key)| plural(key, count as u64))
            .collect::<Vec<_>>();
            group.summary = vec![(None, DisplayText::plain(parts.join(", ")))];
        }
        self.records.set_groups(records);
    }

    fn confirm_delete(
        &mut self,
        ids: Vec<String>,
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled() {
            return;
        }
        self.pending_delete = Some(ids);
        dialog::confirm(
            t!("records.delete_title"),
            message,
            t!("common.delete"),
            Self::delete,
            |this, cx| {
                this.pending_delete = None;
                this.notify(cx);
            },
            window,
            cx,
        );
        self.notify(cx);
    }

    fn delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(ids) = self.pending_delete.take() else {
            return;
        };
        self.deleting = ids.iter().cloned().collect();
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let count = stock::delete_stocks(&app, ids).await?;
                Ok(plural("records.deleted", count).into_owned())
            },
            |this, result, window, cx| {
                this.deleting.clear();
                if this.status.saved(result) {
                    this.context.years_changed(cx);
                    window.focus(&this.focus, cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    /// Includes the selected records the filter hides.
    fn delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = &self.records.selected;
        let message = delete_selected_message(selected, self.records.visible());
        let ids = selected.iter().cloned().collect();
        self.confirm_delete(ids, message, window, cx);
    }

    fn open_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        let year = self
            .year
            .or(self.missing_rate_year)
            .unwrap_or_else(|| tax_date(Utc::now()).year());
        self.nbp_year = Some(nbp::open(year, Self::nbp_dialog, window, cx));
        self.status.error = None;
        self.status.message = None;
        self.notify(cx);
    }

    fn close_nbp(&mut self, cx: &mut Context<Self>) {
        self.nbp_year = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn import_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(input) = &self.nbp_year else {
            return;
        };
        let year = match nbp::year(input, cx) {
            Ok(year) => year,
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
            move |app| nbp::import(app, year),
            |this, result, window, cx| {
                if this.status.saved(result) {
                    window.close_dialog(cx);
                    this.close_nbp(cx);
                    window.focus(&this.focus, cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn nbp_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(input) = &self.nbp_year else {
            return dialog;
        };
        nbp::dialog(
            self,
            dialog,
            input,
            |this| &this.status,
            Self::import_nbp,
            Self::close_nbp,
            cx,
        )
    }

    fn header(&self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let disabled = self.disabled();
        let filterable = self.status.ready && self.missing_rate.is_none() && !self.is_empty();
        let narrow = self.width().is_some_and(|width| width < px(NARROW));
        let (shown, total) = match self.tab {
            Tab::Sales => (self.sales.shown(), self.sales.state.rows.len()),
            Tab::Realized => self
                .display
                .as_ref()
                .and_then(|display| display.portfolio.as_ref())
                .map_or((0, 0), |portfolio| portfolio.realized.counts(&self.query)),
            Tab::Records => (self.records.shown(), self.records.state.rows.len()),
            Tab::Holdings => {
                self.display
                    .as_ref()
                    .map_or((0, 0), |display| match &display.portfolio {
                        Some(portfolio) => {
                            (portfolio.shown(&self.query), portfolio.positions.len())
                        }
                        None => (
                            holdings::shown(display, &self.query),
                            display.holdings.len(),
                        ),
                    })
            }
        };
        let count = (!self.query.is_empty()).then(|| filter_count(shown, total, narrow));
        let meta = match (&self.display, self.year) {
            (Some(display), None) => format!(
                "{} · {}",
                header::year_label(None),
                plural("count.stocks", display.stocks.len() as u64)
            )
            .into(),
            _ => header::year_label(self.year),
        };
        header::page(Page::Stocks.title(), Some(meta), window, cx)
            .when(filterable, |row| {
                row.child(filter_field(
                    &self.filter,
                    count,
                    self.editor.is_some() || self.nbp_year.is_some(),
                    cx,
                ))
            })
            .child(
                header::actions()
                    .child(self.status.refreshing(cx))
                    .child(header::refresh(disabled, cx).on_click(cx.listener(
                        |this, _, window, cx| {
                            this.context.years_changed(cx);
                            this.refresh(window, cx);
                        },
                    )))
                    .child(import_button(narrow).disabled(disabled).on_click(
                        cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx)),
                    ))
                    .child(
                        self.add_button("add-record", Some(IconName::Plus), disabled, cx)
                            .primary(),
                    ),
            )
    }

    fn add_button(
        &self,
        id: &'static str,
        icon: Option<IconName>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        header::button(id, icon, t!("records.add"))
            .disabled(disabled)
            .on_click(cx.listener(|this, _, window, cx| this.open_editor(None, None, window, cx)))
    }

    fn stats(&self, compact: bool, cx: &App) -> Div {
        if let Some(portfolio) = self
            .display
            .as_ref()
            .and_then(|display| display.portfolio.as_ref())
        {
            return portfolio.stats(compact, cx);
        }
        let stats = self.display.iter().flat_map(|display| &display.stats);
        totals_row(compact).children(stats.map(|(label, value)| {
            value::stat(
                SharedString::from(format!("total-{label}")),
                t!(*label),
                value,
                cx,
            )
        }))
    }

    fn open_set_exchange(
        &mut self,
        rows: Vec<ExchangeRow>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled() || rows.is_empty() {
            return;
        }
        let dialog = ExchangeDialog::open(
            self.context.services.clone(),
            self.catalog.clone(),
            t!("stocks.set_exchange"),
            "exchange.description",
            rows,
            window,
            cx,
        );
        let subscription =
            cx.subscribe_in(&dialog, window, |this, _, event, window, cx| match event {
                ExchangeDialogEvent::Saved(message) => {
                    this.exchange_dialog = None;
                    this.notify(cx);
                    this.refresh(window, cx);
                    this.status.message = Some(message.clone().into());
                }
                ExchangeDialogEvent::Closed => {
                    this.exchange_dialog = None;
                    this.notify(cx);
                }
            });
        self.exchange_dialog = Some((dialog, subscription));
        self.status.error = None;
        self.status.message = None;
        self.notify(cx);
    }

    fn warning(&self, index: usize, card: &WarningCard, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let action = card.action.clone();
        let label = match action {
            WarningAction::AddBuy(_) => t!("stocks.add_buy"),
            WarningAction::SetExchange(_) => t!("stocks.set_exchange"),
        };
        let button = header::button(
            SharedString::from(format!("warning-action-{index}")),
            None,
            label,
        )
        .disabled(self.disabled())
        .on_click(cx.listener(move |this, _, window, cx| match &action {
            WarningAction::AddBuy(prefill) => this.open_editor(None, prefill.clone(), window, cx),
            WarningAction::SetExchange(rows) => this.open_set_exchange(rows.clone(), window, cx),
        }));
        h_flex()
            .flex_shrink_0()
            .mx(px(20.))
            .mb(px(8.))
            .pl(px(12.))
            .pr(px(8.))
            .py(px(8.))
            .gap(px(10.))
            .rounded(px(10.))
            .border_1()
            .border_color(p.warning.opacity(0.28))
            .bg(p.warning.opacity(0.08))
            .child(
                Icon::new(card.icon)
                    .size(px(16.))
                    .flex_shrink_0()
                    .text_color(p.warning),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .flex_wrap()
                    .gap_x(px(10.))
                    .child(div().font_semibold().child(card.title.clone()))
                    .child(
                        div()
                            .min_w_0()
                            .text_color(p.muted)
                            .child(card.description.clone()),
                    ),
            )
            .child(button)
    }

    fn tabs(&self, cx: &mut Context<Self>) -> Div {
        let Some(display) = &self.display else {
            return div();
        };
        if let Some(portfolio) = &display.portfolio {
            return portfolio.tabs(
                self.tab,
                display.stocks.len(),
                cx.listener(|this, tab, _, cx| this.select_tab(*tab, cx)),
                cx,
            );
        }
        let year = self.year.unwrap_or_default();
        let caption = match self.tab {
            Tab::Sales | Tab::Realized => t!("stocks.sales_caption"),
            Tab::Records => t!("stocks.records_caption", year = year),
            Tab::Holdings => t!("stocks.holdings_caption", year = year),
        };
        let items = vec![
            TabItem {
                key: Tab::Sales,
                label: t!("stocks.sales").into(),
                count: self.sales.state.rows.len(),
            },
            TabItem {
                key: Tab::Records,
                label: t!("stocks.records").into(),
                count: self.records.state.rows.len(),
            },
            TabItem {
                key: Tab::Holdings,
                label: t!("stocks.holdings").into(),
                count: display.holdings.len(),
            },
        ];
        tabs::tabs(
            "stock-tabs",
            t!("stocks.view"),
            items,
            self.tab,
            cx.listener(|this, tab, _, cx| this.select_tab(*tab, cx)),
            caption,
            cx,
        )
    }

    fn tab_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.year.is_none() {
            return match self.tab {
                Tab::Holdings => self.portfolio_holdings(cx),
                Tab::Realized => self.realized_view(cx),
                Tab::Records => {
                    let empty = no_match(&self.query, cx);
                    v_flex()
                        .flex_1()
                        .min_h_0()
                        .child(table(self, Table::Records, empty, window, cx))
                        .child(
                            notice::info(t!("stocks.all_records_note"), cx)
                                .mx(px(20.))
                                .my(px(12.))
                                .flex_shrink_0(),
                        )
                        .into_any_element()
                }
                Tab::Sales => div().into_any_element(),
            };
        }
        let year = self.year.unwrap_or_default();
        let empty = |rows: usize, none: SharedString, cx: &App| {
            if rows == 0 {
                div()
                    .px(px(20.))
                    .py(px(24.))
                    .text_color(palette(cx).muted)
                    .child(none)
            } else {
                no_match(&self.query, cx)
            }
        };
        match self.tab {
            Tab::Realized => div().into_any_element(),
            Tab::Sales => {
                let empty = empty(
                    self.sales.state.rows.len(),
                    t!("stocks.no_sales", year = year).into(),
                    cx,
                );
                table(self, Table::Sales, empty, window, cx)
            }
            Tab::Records => {
                let empty = empty(
                    self.records.state.rows.len(),
                    t!("stocks.no_records", year = year).into(),
                    cx,
                );
                table(self, Table::Records, empty, window, cx)
            }
            Tab::Holdings => self.holdings_view(window, cx),
        }
    }

    fn loading(&self, cx: &App) -> Div {
        let visible = self.status.loading_visible;
        let (skeleton, compact) = self.sales.skeleton(visible, cx);
        v_flex()
            .flex_1()
            .min_h_0()
            .child(
                totals_row(compact).children(
                    if self.year.is_some() {
                        vec!["tax.income_c22", "tax.costs_c23"]
                    } else {
                        vec![
                            "stocks.portfolio_cost",
                            "stocks.realized_profit",
                            "stocks.positions",
                            "stocks.sales",
                        ]
                    }
                    .into_iter()
                    .map(|label| value::stat_skeleton(t!(label), visible, cx)),
                ),
            )
            .child(skeleton)
    }

    fn year_link(&self, cx: &mut Context<Self>) -> Option<Button> {
        let disabled = self.disabled();
        self.other_year.map(|(year, count)| {
            let key = plural_key("stocks.other_year", count.into());
            Button::new("other-year")
                .ghost()
                .small()
                .text_color(cx.theme().muted_foreground)
                .disabled(disabled)
                .child(
                    div()
                        .text_size(px(12.))
                        .child(t!(key, year = year, count = count)),
                )
                .child(Icon::new(IconName::ChevronRight).size(px(12.)))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.context.select_year(Some(year), cx)),
                )
        })
    }

    fn import_action(&self, cx: &mut Context<Self>) -> Button {
        header::button("empty-import", None, t!("common.import"))
            .primary()
            .disabled(self.disabled())
            .on_click(cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx)))
    }

    fn empty_state(&self, cx: &mut Context<Self>) -> Empty {
        let year = self.year.unwrap_or_default();
        let providers = IMPORTERS
            .iter()
            .filter(|importer| {
                importer
                    .output
                    .iter()
                    .any(|output| matches!(output, OutputType::Stock))
            })
            .map(|importer| crate::messages::importer_name(importer.kind).to_owned())
            .collect::<Vec<_>>();
        let disabled = self.disabled();
        notice::notice(
            Page::Stocks.icon(),
            false,
            if self.year.is_some() {
                t!("stocks.empty_year", year = year)
            } else {
                t!("stocks.empty_all")
            },
            t!("records.empty_detail", providers = or_list(&providers)),
            cx,
        )
        .content(
            EmptyContent::new()
                .child(
                    h_flex()
                        .gap(px(8.))
                        .mt(px(6.))
                        .child(self.import_action(cx))
                        .child(self.add_button("empty-add", None, disabled, cx)),
                )
                .children(self.year_link(cx)),
        )
    }

    fn missing_rate_state(&self, error: &SharedString, cx: &mut Context<Self>) -> Empty {
        notice::missing_rate(
            self.year,
            self.year.or(self.missing_rate_year),
            error.clone(),
            self.disabled(),
            cx.listener(|this, _, window, cx| this.open_nbp(window, cx)),
            cx.listener(|this, _, _, cx| this.context.navigate(Page::Rates, cx)),
            cx,
        )
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
        let selected = self.year;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let Some(year) = selected else {
                    let portfolio = portfolio_use_case::load_stock_portfolio(&app).await?;
                    return Ok((Some(StockDisplay::all_years(&portfolio)), None));
                };
                let data = stock::load_stocks(&app, year).await?;
                // A sale traded in the previous year can settle in this one.
                let other_year = if data.records.is_empty()
                    && data.tax.sales.is_empty()
                    && data.tax.open_lots.is_empty()
                {
                    let years = year::list_year_info(&app).await.unwrap_or_default();
                    other_year(year, &years)
                } else {
                    None
                };
                let display = StockDisplay::new(&data, year);
                Ok((Some(display), other_year))
            },
            |this, result, _, cx| {
                this.missing_rate_year = result
                    .as_ref()
                    .err()
                    .and_then(|error| error.missing_rate_year());
                let missing_rate = result
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.missing_rate());
                let loaded = this.status.loaded(result);
                this.missing_rate = if missing_rate {
                    this.status.error.take().map(|error| error.text())
                } else {
                    None
                };
                if let Some((display, other_year)) = loaded {
                    this.apply(display, other_year);
                }
                this.context.loaded(cx);
                cx.notify();
            },
        ));
        cx.notify();
    }
}

/// Newer first on a tie.
fn other_year(selected: i32, years: &[YearInfo]) -> Option<(i32, u32)> {
    years
        .iter()
        .filter(|info| info.year != selected && info.stocks > 0)
        .min_by_key(|info| ((info.year - selected).abs(), -info.year))
        .map(|info| (info.year, info.stocks))
}

impl TableHost for StocksPage {
    type Table = Table;

    fn table(&self, which: Table) -> &RecordTable {
        match which {
            Table::Sales => &self.sales,
            Table::Records => &self.records,
        }
    }

    fn table_mut(&mut self, which: Table) -> &mut RecordTable {
        match which {
            Table::Sales => &mut self.sales,
            Table::Records => &mut self.records,
        }
    }

    fn disabled(&self) -> bool {
        self.locked() || self.status.loading
    }

    fn deleting(&self) -> &HashSet<String> {
        &self.deleting
    }

    fn page_focus(&self) -> &FocusHandle {
        &self.focus
    }

    fn editable(&self) -> bool {
        true
    }

    fn copyable(&self) -> bool {
        self.year.is_some()
    }

    fn edit(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let record = self
            .display
            .as_ref()
            .and_then(|display| display.stocks.iter().find(|record| record.id == id))
            .cloned();
        if let Some(record) = record {
            self.open_editor(Some(&record), None, window, cx);
        }
    }

    fn delete_one(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_delete(
            vec![id.to_owned()],
            t!("records.delete_one").into_owned(),
            window,
            cx,
        );
    }

    fn follow(
        &mut self,
        _: Table,
        id: &str,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = self
            .display
            .as_ref()
            .and_then(|display| display.histories.get(id))
            .and_then(|keys| keys.get(index))
            .cloned();
        if let Some(key) = key {
            self.open_history(key, window, cx);
        }
    }
}

impl Render for StocksPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some((history, _)) = &self.history {
            return v_flex().size_full().min_h_0().child(
                div()
                    .id("stocks-page")
                    .track_focus(&self.focus)
                    .tab_index(-1)
                    .size_full()
                    .min_h_0()
                    .child(history.clone()),
            );
        }
        let mut body = v_flex()
            .id("stocks-page")
            .track_focus(&self.focus)
            .tab_index(-1)
            .size_full()
            .min_h_0()
            // overflow_y_scrollbar() wraps its content in h_auto(), which
            // unbounds the virtual list below.
            .overflow_y_scroll()
            .lock_scroll_axis()
            .track_scroll(&self.page_scroll)
            // The editor and the NBP dialog show their own errors.
            .when(
                self.status.is_visible() && self.editor.is_none() && self.nbp_year.is_none(),
                |view| {
                    view.child(
                        div()
                            .flex_shrink_0()
                            .px(px(20.))
                            .pb_3()
                            .child(self.status.render(|this| &mut this.status, cx)),
                    )
                },
            );
        if let Some(error) = self.missing_rate.clone() {
            body = body.child(self.missing_rate_state(&error, cx));
        } else if self.status.ready && self.is_empty() {
            body = body.child(self.empty_state(cx));
        } else if self.status.ready {
            // Sales decides for every tab, so the totals don't move when the tab changes.
            let compact = TableLayout::new(
                &self.sales.columns,
                self.sales.state.fit.as_deref().unwrap_or_default(),
                self.width().unwrap_or(px(900.)),
                false,
            )
            .compact;
            let warnings = self
                .display
                .iter()
                .flat_map(|display| display.warnings.iter().enumerate())
                .map(|(index, card)| self.warning(index, card, cx))
                .collect::<Vec<_>>();
            body = body
                .child(self.stats(compact, cx))
                .children(warnings)
                .child(self.tabs(cx))
                .child(self.tab_content(window, cx));
        } else if self.status.loading {
            body = body.child(self.loading(cx));
        } else {
            let disabled = self.status.busy || self.pending_delete.is_some();
            body = body.child(
                h_flex().px(px(20.)).gap_2().child(
                    Button::new("retry")
                        .text_label(t!("common.retry"))
                        .outline()
                        .disabled(disabled)
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            );
        }
        let selecting =
            self.status.ready && self.tab == Tab::Records && !self.records.selected.is_empty();
        v_flex()
            .size_full()
            .min_h_0()
            .child(self.header(window, cx))
            .child(
                v_flex()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(body)
                    .vertical_scrollbar(&self.page_scroll)
                    .when(selecting, |view| {
                        view.child(selection_bar(
                            self,
                            Table::Records,
                            Self::delete_selected,
                            cx,
                        ))
                    }),
            )
    }
}
