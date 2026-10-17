use super::{StocksPage, display::trade_date};
use crate::{
    components::{
        chart::{BarChart, chart_tooltip, section, so_far},
        measure_width, notice, records, scroll_table, value,
    },
    format::{DisplayText, count, money, pln, shares},
    pages::records::TableHost,
    theme::{palette, tabular_digits},
};
use chrono::{Datelike, Utc};
use gpui_kit::{
    assets::IconName,
    component::{tooltip::Tooltip, *},
    prelude::FluentBuilder,
    *,
};
use pitpls_app::use_case::portfolio::StockPortfolio;
use pitpls_core::time::tax_date;
use rust_decimal::Decimal;
use rust_i18n::t;
use std::collections::BTreeSet;

pub struct Layout {
    pub width: Option<Pixels>,
    pan: ScrollHandle,
    open: BTreeSet<(SharedString, SharedString)>,
}

impl Layout {
    pub fn new(width: Option<Pixels>) -> Self {
        Self {
            width,
            pan: ScrollHandle::default(),
            open: BTreeSet::new(),
        }
    }
}

struct Instrument {
    ticker: SharedString,
    exchange: Option<SharedString>,
    provider: SharedString,
    search: String,
    sales: BTreeSet<SharedString>,
    shares: DisplayText,
    amounts: [Decimal; 3],
    held: bool,
    tooltip: SharedString,
}

pub struct Display {
    years: Vec<(i32, usize, [Decimal; 3])>,
    instruments: Vec<Instrument>,
}

impl Display {
    pub fn new(data: &StockPortfolio) -> Self {
        Self {
            years: data
                .realized_years
                .iter()
                .map(|year| {
                    (
                        year.year,
                        year.sales,
                        [year.income, year.costs, year.profit],
                    )
                })
                .collect(),
            instruments: data
                .realized_instruments
                .iter()
                .map(|instrument| {
                    let exchange = data
                        .listings
                        .get(&instrument.ticker, &instrument.provider)
                        .map(|listing| SharedString::from(listing.name.clone()));
                    Instrument {
                        ticker: instrument.ticker.clone().into(),
                        provider: instrument.provider.clone().into(),
                        search: format!(
                            "{} {} {}",
                            instrument.ticker,
                            instrument.provider,
                            exchange.as_deref().unwrap_or("")
                        )
                        .to_lowercase(),
                        exchange,
                        sales: data
                            .replay
                            .sales
                            .iter()
                            .filter(|sale| {
                                sale.ticker == instrument.ticker
                                    && sale.provider == instrument.provider
                            })
                            .map(|sale| sale.id.clone().into())
                            .collect(),
                        shares: shares(instrument.shares),
                        amounts: [instrument.income, instrument.costs, instrument.profit],
                        held: instrument.held,
                        tooltip: if instrument.sales == 1 {
                            t!("sale.sold", date = trade_date(instrument.first_sale))
                        } else {
                            t!(
                                "stocks.first_last_sale",
                                first = trade_date(instrument.first_sale),
                                last = trade_date(instrument.last_sale)
                            )
                        }
                        .into(),
                    }
                })
                .collect(),
        }
    }

    pub fn counts(&self, query: &str) -> (usize, usize) {
        (
            self.instruments
                .iter()
                .filter(|row| row.search.contains(query))
                .count(),
            self.instruments.len(),
        )
    }

    fn chart(&self, cx: &App) -> Div {
        let current = tax_date(Utc::now()).year();
        section(
            t!("stocks.realized_by_year"),
            t!("stocks.realized_by_year_caption"),
            BarChart::new(
                "realized-chart",
                self.years.clone(),
                |row| row.0.to_string().into(),
                |row| row.2[2],
                move |row, _, cx| {
                    chart_tooltip(
                        so_far(row.0.to_string(), row.0 == current),
                        vec![
                            (t!("stocks.sales").into(), count(row.1)),
                            (t!("column.income").into(), money(row.2[0])),
                            (t!("column.costs").into(), money(row.2[1])),
                        ],
                        Some((t!("column.profit").into(), pln(row.2[2]))),
                        cx,
                    )
                    .into_any_element()
                },
            )
            .height(px(140.)),
            cx,
        )
    }
}

const WIDTHS: [f32; 9] = [64., 84., 0., 64., 96., 112., 112., 112., 108.];

fn row(cx: &App) -> Div {
    h_flex()
        .w_full()
        .h(px(38.))
        .px(px(12.))
        .gap(px(8.))
        .border_b_1()
        .border_color(palette(cx).line)
        .font_features(tabular_digits())
}

fn slot(index: usize) -> Div {
    div()
        .min_w_0()
        .when_else(
            index == 2,
            |cell| cell.flex_1(),
            |cell| cell.w(px(WIDTHS[index])).flex_shrink_0(),
        )
        .when(matches!(index, 3..=7), |cell| cell.text_right())
}

fn number(id: impl Into<ElementId>, display: &DisplayText, cx: &App) -> Stateful<Div> {
    value::full_tooltip(
        div().id(id).child(value::text(display, true, false, cx)),
        display,
    )
}

impl StocksPage {
    pub fn realized_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(display) = self
            .display
            .as_ref()
            .and_then(|display| display.portfolio.as_ref())
            .map(|portfolio| &portfolio.realized)
        else {
            return div().into_any_element();
        };
        let p = *palette(cx);
        let width = self.realized.width.unwrap_or(px(940.)).max(px(940.));
        let detail_layout = records::TableLayout {
            width,
            frame: [px(0.), px(24.), px(0.)],
            columns: vec![],
            compact: false,
            stacked: false,
        };
        let instruments = display
            .instruments
            .iter()
            .filter(|row| row.search.contains(&self.query))
            .collect::<Vec<_>>();
        let mut rows = v_flex().text_size(px(13.)).child(
            row(cx)
                .h(px(32.))
                .border_t_1()
                .border_color(p.line)
                .text_size(px(12.))
                .text_color(p.faint)
                .whitespace_nowrap()
                .child(div().w(px(16.)).flex_shrink_0())
                .children(
                    [
                        "column.ticker",
                        "column.exchange",
                        "column.provider",
                        "stocks.sales",
                        "stocks.shares_sold",
                        "stocks.income_pln",
                        "stocks.costs_pln",
                        "column.profit_pln",
                        "stocks.now",
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, key)| slot(index).child(t!(key))),
                ),
        );
        for (index, instrument) in instruments.iter().enumerate() {
            let tooltip = instrument.tooltip.clone();
            let key = (instrument.ticker.clone(), instrument.provider.clone());
            let expanded = self.realized.open.contains(&key);
            let instrument_row = row(cx)
                .id(("realized-instrument", index))
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .child(
                    div().w(px(16.)).flex_shrink_0().text_color(p.faint).child(
                        Icon::new(if expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(px(14.)),
                    ),
                )
                .child(slot(0).font_semibold().child(instrument.ticker.clone()))
                .child(
                    slot(1).flex().items_center().children(
                        instrument
                            .exchange
                            .clone()
                            .map(|exchange| value::chip(exchange, false, cx)),
                    ),
                )
                .child(
                    slot(2)
                        .truncate()
                        .text_color(p.muted)
                        .child(instrument.provider.clone()),
                )
                .child(slot(3).child(instrument.sales.len().to_string()))
                .child(slot(4).child(number(("realized-shares", index), &instrument.shares, cx)))
                .children(
                    instrument
                        .amounts
                        .iter()
                        .enumerate()
                        .map(|(column, amount)| {
                            slot(column + 5)
                                .when(column == 2, |cell| cell.font_medium())
                                .child(number(
                                    SharedString::from(format!("realized-{index}-{column}")),
                                    &pln(*amount),
                                    cx,
                                ))
                        }),
                )
                .child(slot(8).flex().items_center().child(value::chip(
                    if instrument.held {
                        t!("stocks.held")
                    } else {
                        t!("stocks.closed")
                    },
                    false,
                    cx,
                )));
            rows = rows.child(
                gpui_kit::base::Button::new(("realized-toggle", index))
                    .role(Role::Button)
                    .accessibility_label(t!(
                        "stocks.toggle_realized",
                        ticker = instrument.ticker,
                        provider = instrument.provider
                    ))
                    .aria_expanded(expanded)
                    .w_full()
                    .text_color(p.text)
                    .when(expanded, |row| row.bg(p.raised))
                    .hover(move |style| style.bg(p.hover))
                    .focus(move |style| style.bg(p.hover))
                    .child(instrument_row)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.disabled() {
                            return;
                        }
                        if !this.realized.open.remove(&key) {
                            this.realized.open.insert(key.clone());
                        }
                        cx.notify();
                    })),
            );
            if expanded {
                rows = rows.children(
                    self.records
                        .state
                        .rows
                        .iter()
                        .filter(|sale| instrument.sales.contains(&sale.id))
                        .map(|sale| {
                            v_flex()
                                .id(SharedString::from(format!("realized-sale-{}", sale.id)))
                                .child(
                                    row(cx)
                                        .h(px(32.))
                                        .pl(detail_layout.indent())
                                        .bg(p.raised)
                                        .font_medium()
                                        .child(t!("sale.sold", date = trade_date(sale.time))),
                                )
                                .child(records::details(
                                    sale,
                                    &detail_layout,
                                    vec![],
                                    None,
                                    [],
                                    false,
                                    cx,
                                ))
                        }),
                );
            }
        }
        if instruments.is_empty() {
            rows = rows.child(div().px(px(12.)).py(px(24.)).text_color(p.muted).child(
                if display.instruments.is_empty() {
                    t!("stocks.no_realized")
                } else {
                    t!("stocks.no_realized_match", query = self.query)
                },
            ));
        }
        let sales = instruments.iter().map(|row| row.sales.len()).sum::<usize>();
        let amounts: [Decimal; 3] =
            std::array::from_fn(|column| instruments.iter().map(|row| row.amounts[column]).sum());
        rows = rows.child(
            row(cx)
                .h(px(40.))
                .font_semibold()
                .child(div().flex_1().child(if self.query.is_empty() {
                    t!("stocks.all")
                } else {
                    t!("stocks.filtered")
                }))
                .child(slot(3).child(sales.to_string()))
                .child(slot(4))
                .children(amounts.iter().enumerate().map(|(index, amount)| {
                    slot(index + 5).child(number(("realized-total", index), &pln(*amount), cx))
                }))
                .child(slot(8)),
        );
        let table =
            scroll_table("realized-table", rows, width, &self.realized.pan).child(measure_width(
                self.realized.width,
                |page: &mut Self| &mut page.realized.width,
                cx,
            ));
        v_flex()
            .flex_shrink_0()
            .gap(px(8.))
            .when(!display.years.is_empty(), |view| {
                view.child(display.chart(cx))
            })
            .child(table)
            .child(
                notice::info(t!("stocks.realized_note"), cx)
                    .mx(px(20.))
                    .mt(px(6.))
                    .mb(px(24.)),
            )
            .into_any_element()
    }
}
