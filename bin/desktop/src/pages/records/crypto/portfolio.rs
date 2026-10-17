use super::super::{
    OpenYear, PortfolioStat, PortfolioView,
    portfolio::{note, scroll_table},
};
use crate::{
    components::{
        chart::{BarChart, chart_tooltip},
        tabs::TabItem,
        year_table::{ColumnWidth, YearTable},
    },
    format::{DisplayText, money, pln},
    theme::palette,
};
use chrono::{Datelike, NaiveDate};
use gpui_kit::{
    component::{StyledExt, h_flex, v_flex},
    *,
};
use pitpls_core::portfolio::{CryptoTotals, CryptoYear};
use rust_decimal::Decimal;
use rust_i18n::t;

#[derive(Default)]
pub struct CryptoPortfolioView {
    years: Vec<CryptoYear>,
    total: CryptoYear,
    put_in: Decimal,
    taken_out: Decimal,
    net: Decimal,
    current_year: i32,
}

impl CryptoPortfolioView {
    pub fn new(years: Vec<CryptoYear>, totals: CryptoTotals, today: NaiveDate) -> Self {
        Self {
            total: CryptoYear {
                buys: years.iter().map(|year| year.buys).sum(),
                bought: years.iter().map(|year| year.bought).sum(),
                sells: years.iter().map(|year| year.sells).sum(),
                sold: years.iter().map(|year| year.sold).sum(),
                fees: years.iter().map(|year| year.fees).sum(),
                net: totals.net,
                ..Default::default()
            },
            years,
            put_in: totals.put_in,
            taken_out: totals.taken_out,
            net: totals.net,
            current_year: today.year(),
        }
    }
}

impl PortfolioView for CryptoPortfolioView {
    const LABELS: &'static [&'static str] = &["crypto.put_in", "crypto.taken_out", "crypto.net"];

    fn stats(&self) -> Vec<PortfolioStat> {
        Self::LABELS
            .iter()
            .zip([self.put_in, self.taken_out, self.net])
            .enumerate()
            .map(|(index, (label, number))| PortfolioStat {
                label,
                value: pln(number),
                unit: Some("PLN".into()),
                caption: match index {
                    0 => Some(t!("crypto.buys_and_fees").into()),
                    1 => Some(t!("crypto.sales").into()),
                    _ => None,
                },
            })
            .collect()
    }

    fn tabs(&self) -> Vec<TabItem<usize>> {
        vec![TabItem {
            key: 0,
            label: t!("crypto.by_year").into(),
            count: self.years.len(),
        }]
    }
    fn caption(&self, tab: Option<usize>) -> SharedString {
        t!(if tab.is_some() {
            "records.open_year_caption"
        } else {
            "crypto.records_caption"
        })
        .into()
    }
    fn callout(&self) -> Option<SharedString> {
        Some(t!("crypto.cash_note").into())
    }
    fn records_note(&self) -> SharedString {
        t!("crypto.records_note").into()
    }

    fn render(
        &self,
        _: usize,
        _: &str,
        scroll: &ScrollHandle,
        width: Pixels,
        on_open: OpenYear,
        cx: &App,
    ) -> AnyElement {
        let current = self.current_year;
        let chart = BarChart::new(
            "crypto-chart",
            self.years
                .iter()
                .map(|year| (year.year, year.bought, year.sold, year.fees, year.net))
                .collect(),
            |year| year.0.to_string().into(),
            |year| year.4,
            move |year, _, cx| {
                let title = if year.0 == current {
                    format!("{} · {}", year.0, t!("year.so_far"))
                } else {
                    year.0.to_string()
                };
                chart_tooltip(
                    title,
                    vec![
                        (t!("crypto.bought").into(), money(year.1)),
                        (t!("crypto.sold").into(), money(year.2)),
                        (t!("crypto.fees").into(), money(year.3)),
                    ],
                    Some((t!("crypto.net").into(), pln(year.4))),
                    cx,
                )
                .into_any_element()
            },
        );
        let table = YearTable::new(vec![
            ColumnWidth::Fixed(px(128.)),
            ColumnWidth::Fixed(px(72.)),
            ColumnWidth::Flexible,
            ColumnWidth::Fixed(px(72.)),
            ColumnWidth::Flexible,
            ColumnWidth::Flexible,
            ColumnWidth::Flexible,
        ]);
        let rows = v_flex()
            .child(
                table.header(
                    [
                        "field.year",
                        "crypto.buys",
                        "crypto.bought_pln",
                        "crypto.sells",
                        "crypto.sold_pln",
                        "crypto.fees_pln",
                        "crypto.net_pln",
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
                    cells(year),
                    selected == current,
                    move |_, window, cx| on_open(selected, window, cx),
                    cx,
                )
            }))
            .child(table.total_row(t!("year.all"), cells(&self.total), cx));
        v_flex()
            .flex_shrink_0()
            .min_w_0()
            .w(width)
            .overflow_hidden()
            .child(
                v_flex()
                    .px(px(20.))
                    .py(px(8.))
                    .child(
                        h_flex()
                            .flex_wrap()
                            .gap_x(px(8.))
                            .pb(px(8.))
                            .text_size(px(12.))
                            .child(div().font_medium().child(t!("crypto.chart_title")))
                            .child(
                                div()
                                    .text_color(palette(cx).faint)
                                    .child(t!("crypto.chart_caption")),
                            ),
                    )
                    .child(chart),
            )
            .child(scroll_table(rows, width.max(px(880.)), scroll))
            .child(note(t!("crypto.year_note"), cx))
            .into_any_element()
    }
}

fn cells(year: &CryptoYear) -> Vec<DisplayText> {
    vec![
        DisplayText::plain(year.buys.to_string()),
        money(year.bought),
        DisplayText::plain(year.sells.to_string()),
        money(year.sold),
        money(year.fees),
        money(year.net),
    ]
}
