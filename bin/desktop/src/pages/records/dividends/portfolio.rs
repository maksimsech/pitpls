use super::super::{
    OpenYear, PortfolioStat, PortfolioView, no_match,
    portfolio::{note, scroll_table},
};
use crate::{
    components::{
        chart::{BarChart, chart_tooltip},
        tabs::TabItem,
        value,
        year_table::{ColumnWidth, YearTable},
    },
    format::{DisplayText, amount, money, pln},
    i18n::plural,
    theme::{palette, tabular_digits},
};
use chrono::{Datelike, NaiveDate};
use gpui_kit::{
    component::{StyledExt, h_flex, tooltip::Tooltip, v_flex},
    prelude::FluentBuilder,
    *,
};
use pitpls_core::portfolio::{DividendPayer, DividendYear};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_i18n::t;

#[derive(Default)]
pub struct DividendPortfolioView {
    years: Vec<DividendYear>,
    payers: Vec<DividendPayer>,
    payments: usize,
    gross: Decimal,
    withheld: Decimal,
    net: Decimal,
    current_year: i32,
}

impl DividendPortfolioView {
    pub fn new(years: Vec<DividendYear>, payers: Vec<DividendPayer>, today: NaiveDate) -> Self {
        Self {
            payments: years.iter().map(|year| year.payments).sum(),
            gross: years.iter().map(|year| year.gross).sum(),
            withheld: years.iter().map(|year| year.withheld).sum(),
            net: years.iter().map(|year| year.net).sum(),
            years,
            payers,
            current_year: today.year(),
        }
    }

    fn by_year(&self, scroll: &ScrollHandle, width: Pixels, on_open: OpenYear, cx: &App) -> Div {
        let current_year = self.current_year;
        let chart = BarChart::new(
            "dividends-chart",
            self.years
                .iter()
                .map(|year| {
                    (
                        year.year,
                        year.payments,
                        year.gross,
                        year.withheld,
                        year.net,
                    )
                })
                .collect(),
            |year| year.0.to_string().into(),
            |year| year.2,
            move |year, _, cx| {
                let title = if year.0 == current_year {
                    format!("{} · {}", year.0, t!("year.so_far"))
                } else {
                    year.0.to_string()
                };
                chart_tooltip(
                    title,
                    vec![
                        (t!("dividends.payments").into(), count(year.1)),
                        (t!("dividends.withheld").into(), money(year.3)),
                        (t!("dividends.net").into(), money(year.4)),
                    ],
                    Some((t!("dividends.gross").into(), pln(year.2))),
                    cx,
                )
                .into_any_element()
            },
        );
        let table = YearTable::new(vec![
            ColumnWidth::Fixed(px(128.)),
            ColumnWidth::Fixed(px(76.)),
            ColumnWidth::Fixed(px(76.)),
            ColumnWidth::Flexible,
            ColumnWidth::Flexible,
            ColumnWidth::Flexible,
        ]);
        let rows = v_flex()
            .child(
                table.header(
                    [
                        "dividends.year",
                        "dividends.payments",
                        "dividends.payers",
                        "dividends.gross_pln",
                        "dividends.withheld_pln",
                        "dividends.net_pln",
                    ]
                    .into_iter()
                    .map(|key| t!(key).into())
                    .collect(),
                    cx,
                ),
            )
            .children(self.years.iter().rev().map(|year| {
                let selected = year.year;
                let on_open = on_open.clone();
                table.year_row(
                    selected,
                    vec![
                        count(year.payments),
                        count(year.payers),
                        money(year.gross),
                        money(year.withheld),
                        money(year.net),
                    ],
                    selected == current_year,
                    move |_, window, cx| on_open(selected, window, cx),
                    cx,
                )
            }))
            .child(table.total_row(
                t!("year.all"),
                vec![
                    count(self.payments),
                    count(self.payers.len()),
                    money(self.gross),
                    money(self.withheld),
                    money(self.net),
                ],
                cx,
            ));
        v_flex()
            .flex_shrink_0()
            .min_w_0()
            .w(width)
            .overflow_hidden()
            .child(
                v_flex()
                    .px(px(20.))
                    .pt(px(8.))
                    .pb(px(8.))
                    .child(
                        h_flex()
                            .flex_wrap()
                            .gap_x(px(8.))
                            .pb(px(8.))
                            .text_size(px(12.))
                            .child(div().font_medium().child(t!("dividends.chart_title")))
                            .child(
                                div()
                                    .text_color(palette(cx).faint)
                                    .child(t!("dividends.chart_caption")),
                            ),
                    )
                    .child(chart),
            )
            .child(scroll_table(rows, width.max(px(800.)), scroll))
            .child(note(t!("dividends.by_year_note"), cx))
    }

    fn by_ticker(&self, query: &str, scroll: &ScrollHandle, width: Pixels, cx: &App) -> Div {
        let p = *palette(cx);
        let mut rows = v_flex().child(
            payer_row(cx)
                .h(px(32.))
                .whitespace_nowrap()
                .text_size(px(12.))
                .text_color(p.faint)
                .children(
                    [
                        "column.ticker",
                        "column.provider",
                        "column.country",
                        "dividends.payments",
                        "dividends.years",
                        "dividends.gross",
                        "dividends.gross_pln",
                        "dividends.withheld_short_pln",
                        "dividends.net_pln",
                        "dividends.share",
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, key)| payer_cell(index).child(t!(key))),
                ),
        );
        let largest = self
            .payers
            .first()
            .map_or(Decimal::ZERO, |payer| payer.gross);
        for (index, payer) in self
            .payers
            .iter()
            .enumerate()
            .filter(|(_, payer)| matches(payer, query))
        {
            let years = if payer.first_year == payer.last_year {
                payer.first_year.to_string()
            } else {
                format!("{}–{}", payer.first_year, payer.last_year)
            };
            let gross = h_flex().justify_end().flex_wrap().gap_x(px(4.)).children(
                payer
                    .gross_in_currency
                    .iter()
                    .enumerate()
                    .map(|(index, gross)| {
                        let display = amount(*gross);
                        h_flex()
                            .gap(px(4.))
                            .when(index > 0, |row| row.child("+"))
                            .child(value::full_tooltip(
                                div()
                                    .id(("gross-currency", index))
                                    .child(value::text(&display, false, true, cx)),
                                &display,
                            ))
                    }),
            );
            rows = rows.child(
                payer_row(cx)
                    .id(("payer", index))
                    .min_h(px(40.))
                    .py(px(8.))
                    .text_size(px(13.))
                    .font_features(tabular_digits())
                    .child(payer_cell(0).font_medium().child(payer.ticker.clone()))
                    .child(
                        payer_cell(1)
                            .text_color(p.muted)
                            .child(payer.provider.clone()),
                    )
                    .child(
                        payer_cell(2)
                            .text_color(p.muted)
                            .child(payer.country.to_string()),
                    )
                    .child(payer_cell(3).child(payer.payments.to_string()))
                    .child(
                        payer_cell(4)
                            .whitespace_nowrap()
                            .text_color(p.muted)
                            .child(years),
                    )
                    .child(payer_cell(5).child(gross))
                    .children(
                        [payer.gross, payer.withheld, payer.net]
                            .into_iter()
                            .enumerate()
                            .map(|(index, number)| {
                                let display = pln(number);
                                value::full_tooltip(
                                    payer_cell(index + 6)
                                        .id(("payer-value", index))
                                        .when(index == 2, |cell| cell.font_medium())
                                        .child(value::text(&display, true, false, cx)),
                                    &display,
                                )
                            }),
                    )
                    .child(payer_cell(9).child(share(payer.gross, largest, self.gross, cx))),
            );
        }
        let none = !self.payers.iter().any(|payer| matches(payer, query));
        v_flex()
            .flex_shrink_0()
            .min_w_0()
            .w(width)
            .overflow_hidden()
            .child(scroll_table(rows, width.max(px(948.)), scroll))
            .when(none, |view| view.child(no_match(query, cx)))
            .child(note(t!("dividends.by_ticker_note"), cx))
    }
}

impl PortfolioView for DividendPortfolioView {
    const LABELS: &'static [&'static str] = &[
        "dividends.gross",
        "dividends.withheld",
        "dividends.net",
        "dividends.payers",
    ];

    fn stats(&self) -> Vec<PortfolioStat> {
        let mut stats = Self::LABELS[..3]
            .iter()
            .zip([self.gross, self.withheld, self.net])
            .map(|(label, number)| PortfolioStat {
                label,
                value: pln(number),
                unit: Some("PLN".into()),
                caption: None,
            })
            .collect::<Vec<_>>();
        stats.push(PortfolioStat {
            label: Self::LABELS[3],
            value: count(self.payers.len()),
            unit: None,
            caption: self.years.first().map(|year| {
                t!(
                    "dividends.payments_since",
                    payments = plural("dividends.payments_count", self.payments as u64),
                    year = year.year
                )
                .into()
            }),
        });
        stats
    }

    fn tabs(&self) -> Vec<TabItem<usize>> {
        vec![
            TabItem {
                key: 0,
                label: t!("dividends.by_year").into(),
                count: self.years.len(),
            },
            TabItem {
                key: 1,
                label: t!("dividends.by_ticker").into(),
                count: self.payers.len(),
            },
        ]
    }

    fn caption(&self, tab: Option<usize>) -> SharedString {
        t!(match tab {
            Some(0) => "records.open_year_caption",
            Some(_) => "dividends.by_ticker_caption",
            None => "dividends.records_caption",
        })
        .into()
    }

    fn records_note(&self) -> SharedString {
        t!("dividends.records_note").into()
    }

    fn filtered_count(&self, tab: usize, query: &str) -> Option<(usize, usize)> {
        (tab == 1).then(|| {
            (
                self.payers
                    .iter()
                    .filter(|payer| matches(payer, query))
                    .count(),
                self.payers.len(),
            )
        })
    }

    fn render(
        &self,
        tab: usize,
        query: &str,
        scroll: &ScrollHandle,
        width: Pixels,
        on_open: OpenYear,
        cx: &App,
    ) -> AnyElement {
        match tab {
            0 => self.by_year(scroll, width, on_open, cx),
            _ => self.by_ticker(query, scroll, width, cx),
        }
        .into_any_element()
    }
}

fn count(value: usize) -> DisplayText {
    DisplayText::plain(value.to_string())
}

fn matches(payer: &DividendPayer, query: &str) -> bool {
    format!("{}\n{}\n{}", payer.ticker, payer.provider, payer.country)
        .to_lowercase()
        .contains(query)
}

fn payer_row(cx: &App) -> Div {
    h_flex()
        .gap(px(6.))
        .px(px(10.))
        .border_t_1()
        .border_color(palette(cx).line)
}

fn payer_cell(index: usize) -> Div {
    div()
        .min_w_0()
        .when(index >= 3 && index != 4, |cell| cell.text_right())
        .map(|cell| match index {
            0 => cell.w(px(54.)).flex_shrink_0(),
            1 => cell.w(px(82.)).flex_shrink_0(),
            2 => cell.w(px(50.)).flex_shrink_0(),
            3 => cell.w(px(58.)).flex_shrink_0(),
            4 => cell.w(px(68.)).flex_shrink_0(),
            5 => cell.flex_1().min_w(px(122.)),
            9 => cell.w(px(52.)).flex_shrink_0(),
            _ => cell.flex_1().min_w(px(96.)),
        })
}

fn share(gross: Decimal, largest: Decimal, total: Decimal, cx: &App) -> Stateful<Div> {
    let width = if largest.is_zero() {
        0.
    } else {
        (gross / largest).to_f32().unwrap_or_default().clamp(0., 1.)
    };
    let percentage = money(if total.is_zero() {
        Decimal::ZERO
    } else {
        gross / total * Decimal::ONE_HUNDRED
    });
    div()
        .id("share")
        .w_full()
        .h(px(20.))
        .flex()
        .items_center()
        .role(Role::Image)
        .aria_label(format!(
            "{}{}",
            percentage.full,
            t!("dividends.share_caption")
        ))
        .tooltip(move |window, cx| {
            let percentage = percentage.clone();
            Tooltip::element(move |_, cx| {
                h_flex()
                    .text_size(px(12.))
                    .font_features(tabular_digits())
                    .child(value::text(&percentage, false, false, cx))
                    .child(t!("dividends.share_caption"))
                    .into_any_element()
            })
            .build(window, cx)
        })
        .child(
            div()
                .w(relative(width))
                .h(px(8.))
                .rounded_r(px(3.))
                .bg(palette(cx).series),
        )
}
