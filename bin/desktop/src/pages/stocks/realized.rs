use super::{StocksPage, display::trade_date};
use crate::{
    components::{
        chart::{BarChart, chart_tooltip},
        measure_width, notice, value,
    },
    format::{DisplayText, money, pln, shares},
    theme::{palette, tabular_digits},
};
use chrono::{Datelike, Utc};
use gpui_kit::{
    component::{scroll::ScrollableElement, tag::Tag, tooltip::Tooltip, *},
    prelude::FluentBuilder,
    *,
};
use pitpls_app::use_case::portfolio::StockPortfolio;
use pitpls_core::time::tax_date;
use rust_decimal::Decimal;
use rust_i18n::t;

pub struct Layout {
    pub width: Option<Pixels>,
    pan: ScrollHandle,
}

impl Layout {
    pub fn new(width: Option<Pixels>) -> Self {
        Self {
            width,
            pan: ScrollHandle::default(),
        }
    }
}

struct Instrument {
    ticker: SharedString,
    exchange: Option<SharedString>,
    provider: SharedString,
    search: String,
    sales: usize,
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
                        sales: instrument.sales,
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
        v_flex()
            .flex_shrink_0()
            .child(
                h_flex()
                    .px(px(20.))
                    .pt(px(6.))
                    .pb(px(8.))
                    .gap(px(8.))
                    .flex_wrap()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_semibold()
                            .child(t!("stocks.realized_by_year")),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(palette(cx).faint)
                            .child(t!("stocks.realized_by_year_caption")),
                    ),
            )
            .child(
                BarChart::new(
                    "realized-chart",
                    self.years.clone(),
                    |row| row.0.to_string().into(),
                    |row| row.2[2],
                    move |row, _, cx| {
                        let title = if row.0 == current {
                            format!("{} · {}", row.0, t!("year.so_far"))
                        } else {
                            row.0.to_string()
                        };
                        chart_tooltip(
                            title,
                            vec![
                                (
                                    t!("stocks.sales").into(),
                                    DisplayText::plain(row.1.to_string()),
                                ),
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
            )
    }
}

const WIDTHS: [f32; 9] = [64., 84., 0., 64., 96., 124., 124., 124., 88.];

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
        let instruments = display
            .instruments
            .iter()
            .filter(|row| row.search.contains(&self.query))
            .collect::<Vec<_>>();
        let mut rows = v_flex().w(width).text_size(px(13.)).child(
            row(cx)
                .h(px(32.))
                .border_t_1()
                .border_color(p.line)
                .text_size(px(12.))
                .text_color(p.faint)
                .whitespace_nowrap()
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
            rows = rows.child(
                row(cx)
                    .id(("realized-instrument", index))
                    .hover(move |style| style.bg(p.hover))
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                    .child(slot(0).font_semibold().child(instrument.ticker.clone()))
                    .child(
                        slot(1).children(instrument.exchange.clone().map(|exchange| {
                            Tag::custom(transparent_black(), p.muted, p.strong_line)
                                .h(px(20.))
                                .px(px(7.))
                                .py_0()
                                .rounded(px(6.))
                                .text_size(px(12.))
                                .child(exchange)
                        })),
                    )
                    .child(
                        slot(2)
                            .truncate()
                            .text_color(p.muted)
                            .child(instrument.provider.clone()),
                    )
                    .child(slot(3).child(instrument.sales.to_string()))
                    .child(slot(4).child(number(
                        ("realized-shares", index),
                        &instrument.shares,
                        cx,
                    )))
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
                    .child(
                        slot(8).child(
                            Tag::custom(transparent_black(), p.muted, p.strong_line)
                                .h(px(20.))
                                .px(px(7.))
                                .py_0()
                                .rounded(px(6.))
                                .text_size(px(12.))
                                .child(if instrument.held {
                                    t!("stocks.held")
                                } else {
                                    t!("stocks.closed")
                                }),
                        ),
                    ),
            );
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
        let sales = instruments.iter().map(|row| row.sales).sum::<usize>();
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
        let table = div()
            .id("realized-table")
            .w_full()
            .min_w_0()
            .relative()
            .flex_shrink_0()
            .overflow_hidden()
            .child(measure_width(
                self.realized.width,
                |page: &mut Self| &mut page.realized.width,
                cx,
            ))
            .child(
                div()
                    .id("realized-pan")
                    .w_full()
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.realized.pan)
                    .child(rows),
            )
            .horizontal_scrollbar(&self.realized.pan);
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
