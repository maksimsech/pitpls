mod display;
mod holdings;

use crate::{
    components::{
        ButtonText, Status, dialog, header, nbp, notice,
        records::{CellStyle, RecordColumn},
        value,
    },
    format::{DisplayText, pln, record_count},
    navigation::{Page, PageContext},
    pages::records::{
        RecordTable, TableHost, delete_selected_message, filter_count, filter_field, import_button,
        no_match, selection_bar, table, totals_row,
    },
    theme::{palette, tabular_digits},
};
use display::{Action, StockDisplay, WarningCard};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    Role,
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
use pitpls_app::use_case::{
    stock,
    year::{self, YearInfo},
};
use pitpls_importers::{IMPORTERS, OutputType};
use rust_decimal::Decimal;
use std::collections::HashSet;

const NARROW: f32 = 600.;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Sales,
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
    filter: Entity<InputState>,
    query: String,
    page_scroll: ScrollHandle,
    pending_delete: Option<Vec<String>>,
    deleting: HashSet<String>,
    nbp_year: Option<Entity<InputState>>,
    missing_rate: Option<SharedString>,
    other_year: Option<(i32, u32)>,
    focus: FocusHandle,
    _subscriptions: [Subscription; 2],
}

fn sale_columns() -> Vec<RecordColumn> {
    vec![
        RecordColumn::new("Date", CellStyle::Time, 96., 50.),
        RecordColumn::new("Ticker", CellStyle::Strong, 64., 56.),
        RecordColumn::new("Exchange", CellStyle::Muted, 80., 64.),
        RecordColumn::grow("Provider", CellStyle::Muted),
        RecordColumn::new("Shares", CellStyle::Number, 64., 56.),
        RecordColumn::new("Income", CellStyle::Number, 130., 110.),
        RecordColumn::new("Costs", CellStyle::Number, 130., 110.).optional(),
        RecordColumn::new("Profit", CellStyle::Number, 130., 110.),
    ]
}

fn record_columns() -> Vec<RecordColumn> {
    vec![
        RecordColumn::new("Date", CellStyle::Time, 96., 50.),
        RecordColumn::new("Action", CellStyle::Tag, 64., 56.),
        RecordColumn::new("Ticker", CellStyle::Strong, 64., 56.),
        RecordColumn::new("Exchange", CellStyle::Muted, 80., 64.).optional(),
        RecordColumn::grow("Provider", CellStyle::Muted),
        RecordColumn::new("Shares", CellStyle::Number, 64., 56.),
        RecordColumn::new("Price", CellStyle::Number, 104., 90.).optional(),
        RecordColumn::new("Value", CellStyle::Number, 120., 100.),
        RecordColumn::new("Fee", CellStyle::Number, 90., 80.).optional(),
    ]
}

fn counted(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

impl StocksPage {
    pub fn new(
        context: PageContext,
        year: Option<i32>,
        width: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter"));
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
        let mut view = Self {
            context,
            year,
            status: Status::default(),
            tab: Tab::Sales,
            display: None,
            sales: RecordTable::new("stock-sales", sale_columns(), false, width),
            records: RecordTable::new("stock-records", record_columns(), true, width),
            holdings: holdings::Layout::new(width),
            filter,
            query: String::new(),
            page_scroll: ScrollHandle::default(),
            pending_delete: None,
            deleting: HashSet::new(),
            nbp_year: None,
            missing_rate: None,
            other_year: None,
            focus: cx.focus_handle(),
            _subscriptions: [filter_subscription, focus_subscription],
        };
        view.refresh(window, cx);
        view
    }

    fn locked(&self) -> bool {
        self.status.busy || self.pending_delete.is_some() || self.nbp_year.is_some()
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(self.locked(), cx);
        cx.notify();
    }

    fn width(&self) -> Option<Pixels> {
        match self.tab {
            Tab::Sales => self.sales.state.viewport_width,
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
            Tab::Records => self.records.state.viewport_width = width,
            Tab::Holdings => self.holdings.width = width,
        }
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
            group.count = counted(group.records.len(), "sale", "sales").into();
            let profit = group
                .records
                .iter()
                .map(|&index| display.profits[index])
                .sum::<Decimal>();
            group.summary = vec![(Some("profit"), pln(profit))];
        }
        self.sales.set_groups(sales);

        let mut records = self.records.month_groups(&self.query, false);
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
                (count(Action::Buy), "buy", "buys"),
                (count(Action::Sell), "sale", "sales"),
                (count(Action::Split), "split", "splits"),
            ]
            .into_iter()
            .filter(|(count, _, _)| *count > 0)
            .map(|(count, one, many)| counted(count, one, many))
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
            "Delete records",
            message,
            "Delete",
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
                Ok(format!("Deleted {count} record(s)."))
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
            .unwrap_or_else(|| chrono::Datelike::year(&chrono::Local::now()));
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
        let filterable = self.status.ready
            && self.missing_rate.is_none()
            && self
                .display
                .as_ref()
                .is_some_and(|display| !display.is_empty());
        let narrow = self.width().is_some_and(|width| width < px(NARROW));
        let (shown, total) = match self.tab {
            Tab::Sales => (self.sales.shown(), self.sales.state.rows.len()),
            Tab::Records => (self.records.shown(), self.records.state.rows.len()),
            Tab::Holdings => self.display.as_ref().map_or((0, 0), |display| {
                (
                    holdings::shown(display, &self.query),
                    display.holdings.len(),
                )
            }),
        };
        let count = (!self.query.is_empty()).then(|| filter_count(shown, total, narrow));
        header::page(
            Page::Stocks.title(),
            Some(header::year_label(self.year)),
            window,
            cx,
        )
        .when(filterable, |row| {
            row.child(filter_field(
                &self.filter,
                count,
                self.nbp_year.is_some(),
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
                )),
        )
    }

    fn stats(&self, compact: bool, cx: &App) -> Div {
        let stats = self.display.iter().flat_map(|display| &display.stats);
        totals_row(compact).children(stats.map(|(label, value)| {
            value::stat(
                SharedString::from(format!("total-{label}")),
                *label,
                value,
                cx,
            )
        }))
    }

    fn warning(card: &WarningCard, cx: &App) -> Div {
        let p = *palette(cx);
        h_flex()
            .flex_shrink_0()
            .mx(px(20.))
            .mb(px(8.))
            .px(px(12.))
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
    }

    fn tabs(&self, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let Some(display) = &self.display else {
            return div();
        };
        let year = self.year.unwrap_or_default();
        let caption = match self.tab {
            Tab::Sales => {
                "Oldest shares first (FIFO) · NBP rate of the working day before settlement"
                    .to_owned()
            }
            Tab::Records => {
                format!("Everything you imported or added for {year}, buys and splits included")
            }
            Tab::Holdings => format!(
                "Open lots on 31.12.{year} · check them against your broker's year-end statement"
            ),
        };
        let tabs = [
            (Tab::Sales, "Sales", self.sales.state.rows.len()),
            (Tab::Records, "Records", self.records.state.rows.len()),
            (Tab::Holdings, "Holdings", display.holdings.len()),
        ];
        h_flex()
            .flex_shrink_0()
            .h(px(40.))
            .px(px(20.))
            .pb(px(8.))
            .gap(px(12.))
            .child(
                h_flex()
                    .id("stock-tabs")
                    .flex_shrink_0()
                    .role(Role::TabList)
                    .aria_label("Stocks view")
                    .gap(px(2.))
                    .p(px(2.))
                    .rounded(px(9.))
                    .bg(p.selected)
                    .children(tabs.map(|(tab, label, count)| {
                        let selected = self.tab == tab;
                        Button::new(SharedString::from(format!("tab-{label}")))
                            .ghost()
                            .role(Role::Tab)
                            .h(px(24.))
                            .px(px(10.))
                            .gap(px(6.))
                            .rounded(px(7.))
                            .accessibility_label(format!("{label}, {count}"))
                            .when(selected, |tab| tab.bg(cx.theme().tokens.button).shadow_xs())
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_medium()
                                    .text_color(if selected { p.text } else { p.muted })
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .font_features(tabular_digits())
                                    .child(count.to_string()),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| this.select_tab(tab, cx)))
                    })),
            )
            .child(div().flex_1())
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(caption),
            )
    }

    fn tab_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let year = self.year.unwrap_or_default();
        let empty = |rows: usize, none: String, cx: &App| {
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
            Tab::Sales => {
                let empty = empty(
                    self.sales.state.rows.len(),
                    format!("No sales in {year}."),
                    cx,
                );
                table(self, Table::Sales, empty, window, cx)
            }
            Tab::Records => {
                let empty = empty(
                    self.records.state.rows.len(),
                    format!("No records in {year}."),
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
                    ["Income (C-22)", "Costs (C-23)", "Tax paid abroad (D-34)"]
                        .map(|label| value::stat_skeleton(label, visible, cx)),
                ),
            )
            .child(skeleton)
    }

    fn year_link(&self, cx: &mut Context<Self>) -> Option<Button> {
        let disabled = self.disabled();
        self.other_year.map(|(year, count)| {
            let records = if count == 1 { "record" } else { "records" };
            Button::new("other-year")
                .ghost()
                .small()
                .text_color(cx.theme().muted_foreground)
                .disabled(disabled)
                .child(div().text_size(px(12.)).child(format!(
                    "{year} has {count} stock {records} · switch to {year}"
                )))
                .child(Icon::new(IconName::ChevronRight).size(px(12.)))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.context.select_year(Some(year), cx)),
                )
        })
    }

    fn import_action(&self, cx: &mut Context<Self>) -> Button {
        header::button("empty-import", None, "Import")
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
            .map(|importer| crate::messages::importer_name(importer.kind))
            .collect::<Vec<_>>()
            .join(" or ");
        notice::notice(
            Page::Stocks.icon(),
            false,
            format!("No stocks in {year}"),
            format!("Import a {providers} statement."),
            cx,
        )
        .content(
            EmptyContent::new()
                .child(h_flex().mt(px(6.)).child(self.import_action(cx)))
                .children(self.year_link(cx)),
        )
    }

    // TODO: Revisit with the all-years page; until then stocks are only
    // calculated for a chosen tax year (docs/stocks/README.md, decision 11).
    fn all_years_state(&self, cx: &mut Context<Self>) -> Empty {
        notice::notice(
            Page::Stocks.icon(),
            false,
            "Pick a tax year".into(),
            "Stocks are calculated one tax year at a time: its sales, its records and the lots \
             open on 31.12.",
            cx,
        )
        .content(EmptyContent::new().children(self.year_link(cx)))
    }

    fn missing_rate_state(&self, error: &SharedString, cx: &mut Context<Self>) -> Empty {
        notice::missing_rate(
            self.year,
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
                    let years = year::list_year_info(&app).await?;
                    return Ok((None, latest_year(&years)));
                };
                let data = stock::load_stocks(&app, year).await?;
                let display = StockDisplay::new(&data, year);
                let other_year = if display.is_empty() {
                    let years = year::list_year_info(&app).await.unwrap_or_default();
                    other_year(year, &years)
                } else {
                    None
                };
                Ok((Some(display), other_year))
            },
            |this, result, _, cx| {
                let missing_rate = result
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.missing_rate());
                let loaded = this.status.loaded(result);
                this.missing_rate = if missing_rate {
                    this.status.error.take()
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

fn latest_year(years: &[YearInfo]) -> Option<(i32, u32)> {
    years
        .iter()
        .filter(|info| info.stocks > 0)
        .max_by_key(|info| info.year)
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
        false
    }

    // Stocks get their form in docs/stocks/09-add-record.md.
    fn edit(&mut self, _: &str, _: &mut Window, _: &mut Context<Self>) {}

    fn delete_one(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_delete(
            vec![id.to_owned()],
            "Delete this record? This cannot be undone.".into(),
            window,
            cx,
        );
    }
}

impl Render for StocksPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            // The NBP dialog shows its own errors.
            .when(
                self.status.is_visible() && self.nbp_year.is_none(),
                |view| {
                    view.child(
                        div()
                            .flex_shrink_0()
                            .px(px(20.))
                            .pb_3()
                            .child(self.status.render()),
                    )
                },
            );
        let empty = self.display.as_ref().is_none_or(StockDisplay::is_empty);
        if let Some(error) = self.missing_rate.clone() {
            body = body.child(self.missing_rate_state(&error, cx));
        } else if self.status.ready && self.year.is_none() {
            body = body.child(self.all_years_state(cx));
        } else if self.status.ready && empty {
            body = body.child(self.empty_state(cx));
        } else if self.status.ready {
            let compact = match self.tab {
                Tab::Sales => self.sales.compact(),
                Tab::Records => self.records.compact(),
                Tab::Holdings => self.holdings.compact(),
            };
            let warnings = self
                .display
                .iter()
                .flat_map(|display| &display.warnings)
                .map(|card| Self::warning(card, cx))
                .collect::<Vec<_>>();
            body = body
                .child(self.stats(compact, cx))
                .children(warnings)
                .child(self.tabs(cx))
                .child(self.tab_content(window, cx));
        } else if self.status.loading {
            body = body.when(self.year.is_some(), |body| body.child(self.loading(cx)));
        } else {
            let disabled = self.status.busy || self.pending_delete.is_some();
            body = body.child(
                h_flex().px(px(20.)).gap_2().child(
                    Button::new("retry")
                        .text_label("Retry")
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
