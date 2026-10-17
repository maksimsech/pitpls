use super::super::{
    OpenYear, PortfolioStat, PortfolioView, no_match, portfolio::note, table::month_title,
};
use crate::{
    components::{
        chart::{BarChart, chart_tooltip, section, so_far},
        scroll_table,
        tabs::TabItem,
        value,
        year_table::{ColumnWidth, YearTable},
    },
    format::{DisplayText, amount, count, date, money, pln},
    i18n::plural,
    theme::{palette, tabular_digits},
};
use chrono::{Datelike, NaiveDate};
use gpui_kit::{
    component::{StyledExt, h_flex, v_flex},
    prelude::FluentBuilder,
    *,
};
use pitpls_core::{
    portfolio::{InterestMonth, InterestProvider, InterestYear, Last12Months},
    time,
};
use rust_decimal::Decimal;
use rust_i18n::t;

pub struct InterestPortfolioView {
    months: Vec<InterestMonth>,
    years: Vec<InterestYear>,
    providers: Vec<InterestProvider>,
    received: Decimal,
    last_12_value: Decimal,
    last_12_from: (i32, u32),
    last_12_to: (i32, u32),
    current_month: (i32, u32),
}

impl InterestPortfolioView {
    pub fn new(
        months: Vec<InterestMonth>,
        years: Vec<InterestYear>,
        providers: Vec<InterestProvider>,
        last_12: Last12Months,
        today: NaiveDate,
    ) -> Self {
        Self {
            received: years.iter().map(|year| year.total).sum(),
            months,
            years,
            providers,
            last_12_value: last_12.value,
            last_12_from: last_12.from,
            last_12_to: last_12.to,
            current_month: (today.year(), today.month()),
        }
    }

    fn by_month(&self, scroll: &ScrollHandle, width: Pixels, on_open: OpenYear, cx: &App) -> Div {
        let current = self.current_month;
        let chart = BarChart::new(
            "interest-chart",
            self.months
                .iter()
                .map(|month| {
                    (
                        (month.year, month.month),
                        month.value,
                        month.providers.clone(),
                    )
                })
                .collect(),
            |month| month_title(month.0, true),
            |month| month.1,
            move |month, _, cx| {
                chart_tooltip(
                    so_far(month_title(month.0, true), month.0 == current),
                    month
                        .2
                        .iter()
                        .map(|(provider, records)| {
                            (
                                provider.clone().into(),
                                DisplayText::plain(plural(
                                    "count.interest_records",
                                    *records as u64,
                                )),
                            )
                        })
                        .collect(),
                    Some((t!("imports.interest").into(), pln(month.1))),
                    cx,
                )
                .into_any_element()
            },
        )
        .months(|month| month.0);
        let mut columns = vec![ColumnWidth::Fixed(px(128.)), ColumnWidth::Fixed(px(76.))];
        columns.extend((0..=self.providers.len()).map(|_| ColumnWidth::Flexible));
        let table = YearTable::new(columns);
        let mut headings = vec![t!("field.year").into(), t!("stocks.records").into()];
        headings.extend(
            self.providers
                .iter()
                .map(|provider| format!("{}, PLN", provider.provider).into()),
        );
        headings.push(t!("home.total_pln").into());
        let rows = v_flex()
            .child(table.header(headings, cx))
            .children(self.years.iter().rev().map(|year| {
                let mut cells = vec![count(year.records)];
                cells.extend(year.by_provider.iter().map(|(_, value)| money(*value)));
                cells.push(money(year.total));
                let selected = year.year;
                let on_open = on_open.clone();
                table.year_row(
                    selected,
                    cells,
                    selected == current.0,
                    move |_, window, cx| on_open(selected, window, cx),
                    cx,
                )
            }))
            .child(table.total_row(
                t!("year.all"),
                {
                    let mut cells = vec![count(self.years.iter().map(|year| year.records).sum())];
                    cells.extend(
                        self.providers
                            .iter()
                            .map(|provider| money(provider.received)),
                    );
                    cells.push(money(self.received));
                    cells
                },
                cx,
            ));
        v_flex()
            .flex_shrink_0()
            .min_w_0()
            .w(width)
            .overflow_hidden()
            .child(section(
                t!("interests.chart_title"),
                t!("interests.chart_caption"),
                chart.height(px(160.)),
                cx,
            ))
            .child(scroll_table(
                "portfolio-table",
                rows,
                width.max(px(252. + 160. * (self.providers.len() + 1) as f32)),
                scroll,
            ))
    }

    fn by_provider(&self, query: &str, scroll: &ScrollHandle, width: Pixels, cx: &App) -> Div {
        let p = *palette(cx);
        let mut rows = v_flex().child(
            provider_row(cx)
                .h(px(32.))
                .text_size(px(12.))
                .text_color(p.faint)
                .children(
                    [
                        "column.provider",
                        "history.currency",
                        "interests.first",
                        "interests.last",
                        "stocks.records",
                        "interests.received",
                        "interests.received_pln",
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, key)| provider_cell(index).child(t!(key))),
                ),
        );
        for (index, provider) in self
            .providers
            .iter()
            .enumerate()
            .filter(|(_, provider)| matches(provider, query))
        {
            let received = h_flex().justify_end().flex_wrap().gap_x(px(4.)).children(
                provider
                    .received_in_currency
                    .iter()
                    .enumerate()
                    .map(|(index, received)| {
                        let display = amount(*received);
                        h_flex()
                            .gap(px(4.))
                            .when(index > 0, |row| row.child("+"))
                            .child(value::full_tooltip(
                                div()
                                    .id(("received-currency", index))
                                    .child(value::text(&display, false, true, cx)),
                                &display,
                            ))
                    }),
            );
            let display = pln(provider.received);
            rows = rows.child(
                provider_row(cx)
                    .id(("provider", index))
                    .min_h(px(40.))
                    .py(px(8.))
                    .text_size(px(13.))
                    .font_features(tabular_digits())
                    .child(
                        provider_cell(0)
                            .font_medium()
                            .child(provider.provider.clone()),
                    )
                    .child(
                        provider_cell(1).text_color(p.muted).child(
                            provider
                                .received_in_currency
                                .iter()
                                .map(|amount| amount.currency.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                    )
                    .child(
                        provider_cell(2)
                            .text_color(p.muted)
                            .child(date(time::tax_date(provider.first)).main),
                    )
                    .child(
                        provider_cell(3)
                            .text_color(p.muted)
                            .child(date(time::tax_date(provider.last)).main),
                    )
                    .child(provider_cell(4).child(provider.records.to_string()))
                    .child(provider_cell(5).child(received))
                    .child(value::full_tooltip(
                        provider_cell(6)
                            .id("received")
                            .font_medium()
                            .child(value::text(&display, true, false, cx)),
                        &display,
                    )),
            );
        }
        v_flex()
            .flex_shrink_0()
            .min_w_0()
            .w(width)
            .overflow_hidden()
            .child(scroll_table(
                "portfolio-table",
                rows,
                width.max(px(860.)),
                scroll,
            ))
            .when(
                !self
                    .providers
                    .iter()
                    .any(|provider| matches(provider, query)),
                |view| view.child(no_match(query, cx)),
            )
            .child(note(t!("interests.provider_note"), cx))
    }
}

impl PortfolioView for InterestPortfolioView {
    const LABELS: &'static [&'static str] = &[
        "interests.received",
        "interests.last_12_months",
        "imports.providers",
    ];

    fn stats(&self) -> Vec<PortfolioStat> {
        vec![
            PortfolioStat {
                label: Self::LABELS[0],
                value: pln(self.received),
                unit: Some("PLN".into()),
                caption: None,
            },
            PortfolioStat {
                label: Self::LABELS[1],
                value: pln(self.last_12_value),
                unit: Some("PLN".into()),
                caption: Some(
                    format!(
                        "· {} – {}",
                        month_number(self.last_12_from),
                        month_number(self.last_12_to)
                    )
                    .into(),
                ),
            },
            PortfolioStat {
                label: Self::LABELS[2],
                value: count(self.providers.len()),
                unit: None,
                caption: self.months.first().map(|month| {
                    t!(
                        "interests.since",
                        month = month_number((month.year, month.month))
                    )
                    .into()
                }),
            },
        ]
    }

    fn tabs(&self) -> Vec<TabItem<usize>> {
        vec![
            TabItem {
                key: 0,
                label: t!("interests.by_month").into(),
                count: self.months.iter().filter(|month| month.records > 0).count(),
            },
            TabItem {
                key: 1,
                label: t!("interests.by_provider").into(),
                count: self.providers.len(),
            },
        ]
    }

    fn caption(&self, tab: Option<usize>) -> SharedString {
        t!(match tab {
            Some(0) => "records.open_year_caption",
            Some(_) => "interests.provider_caption",
            None => "interests.records_caption",
        })
        .into()
    }

    fn records_note(&self) -> SharedString {
        t!("interests.records_note").into()
    }

    fn filtered_count(&self, tab: usize, query: &str) -> Option<(usize, usize)> {
        (tab == 1).then(|| {
            (
                self.providers
                    .iter()
                    .filter(|provider| matches(provider, query))
                    .count(),
                self.providers.len(),
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
            0 => self.by_month(scroll, width, on_open, cx),
            _ => self.by_provider(query, scroll, width, cx),
        }
        .into_any_element()
    }
}

fn month_number((year, month): (i32, u32)) -> String {
    format!("{month:02}.{year}")
}
fn matches(provider: &InterestProvider, query: &str) -> bool {
    provider.provider.to_lowercase().contains(query)
        || provider
            .received_in_currency
            .iter()
            .any(|amount| amount.currency.as_str().to_lowercase().contains(query))
}
fn provider_row(cx: &App) -> Div {
    h_flex()
        .gap(px(12.))
        .px(px(12.))
        .border_t_1()
        .border_color(palette(cx).line)
}
fn provider_cell(index: usize) -> Div {
    div()
        .min_w_0()
        .when(index >= 4, |cell| cell.text_right())
        .map(|cell| match index {
            0 => cell.flex_1().min_w(px(130.)),
            1 => cell.w(px(70.)).flex_shrink_0(),
            2 | 3 => cell.w(px(90.)).flex_shrink_0(),
            4 => cell.w(px(64.)).flex_shrink_0(),
            5 => cell.flex_1().min_w(px(150.)),
            _ => cell.flex_1().min_w(px(130.)),
        })
}
