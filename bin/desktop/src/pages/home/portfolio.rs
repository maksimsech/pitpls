use super::{card, link};
use crate::{
    components::{
        chart::{BarChart, chart_tooltip, fraction, share_bar, share_tooltip, so_far},
        data, notice, value,
        year_table::{ColumnWidth, YearTable},
    },
    format::{money, pln, record_count, shares},
    i18n::{and_list, plural},
    navigation::{Page, PageContext},
    theme::{palette, tabular_digits},
};
use gpui_kit::{assets::IconName, component::*, prelude::FluentBuilder, *};
use pitpls_app::use_case::portfolio::PortfolioSummary;
use pitpls_core::portfolio::Position;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_i18n::t;

const TOTAL_LABELS: [&str; 4] = [
    "home.holdings_cost",
    "home.realized_profit",
    "nav.dividends",
    "imports.interest",
];

pub fn cards(summary: &PortfolioSummary, context: &PageContext, disabled: bool, cx: &App) -> Div {
    let cost: Decimal = summary.positions.iter().map(|position| position.cost).sum();
    v_flex()
        .gap(px(16.))
        .when(
            !summary.positions.is_empty() || !summary.earned.is_empty(),
            |view| view.child(totals(summary, cost, cx)),
        )
        .when(!summary.earned.is_empty(), |view| {
            view.child(by_year(summary, context, disabled, cx))
        })
        .when(!summary.positions.is_empty(), |view| {
            view.child(holdings(summary, cost, context, disabled, cx))
        })
        .when(summary.crypto_records > 0, |view| {
            view.child(crypto(summary, context, disabled, cx))
        })
}

fn totals(summary: &PortfolioSummary, cost: Decimal, cx: &App) -> Div {
    let sales = plural("stocks.sales_count", summary.sales as u64);
    let sales = match summary.first_sale_year {
        Some(year) => t!("home.sales_since", sales = sales, year = year),
        None => sales,
    };
    let captions = [
        format!(
            "{} · {}",
            plural("home.positions_count", summary.positions.len() as u64),
            plural("home.providers_count", summary.holding_providers as u64),
        ),
        sales.into_owned(),
        format!(
            "{} · {}",
            plural("home.payments_count", summary.dividend_payments as u64),
            t!("home.before_tax"),
        ),
        format!(
            "{} · {}",
            plural("count.interest_records", summary.interest_records as u64),
            t!("home.before_tax"),
        ),
    ];
    let values = [cost, summary.realized, summary.dividends, summary.interest];
    data::card(cx).child(
        h_flex().items_stretch().children(
            TOTAL_LABELS
                .into_iter()
                .zip(values)
                .zip(captions)
                .enumerate()
                .map(|(index, ((label, amount), caption))| {
                    let amount = pln(amount);
                    total_cell(index, label, cx)
                        .child(
                            value::full_tooltip(div().id(("portfolio-total", index)), &amount)
                                .flex()
                                .items_baseline()
                                .gap(px(3.))
                                .text_size(px(19.))
                                .font_medium()
                                .font_features(tabular_digits())
                                .child(value::text(&amount, false, false, cx))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(palette(cx).muted)
                                        .child(" PLN"),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(palette(cx).faint)
                                .child(caption),
                        )
                }),
        ),
    )
}

fn total_cell(index: usize, label: &'static str, cx: &App) -> Div {
    v_flex()
        .flex_1()
        .min_w_0()
        .px(px(16.))
        .py(px(14.))
        .gap(px(5.))
        .when(index > 0, |cell| {
            cell.border_l_1().border_color(palette(cx).line)
        })
        .child(
            div()
                .text_size(px(12.))
                .text_color(palette(cx).muted)
                .child(t!(label)),
        )
}

fn by_year(summary: &PortfolioSummary, context: &PageContext, disabled: bool, cx: &App) -> Div {
    let table = YearTable::new(vec![
        ColumnWidth::Fixed(px(128.)),
        ColumnWidth::Flexible,
        ColumnWidth::Flexible,
        ColumnWidth::Flexible,
        ColumnWidth::Flexible,
    ]);
    let current_year = summary.current_year;
    let chart = BarChart::new(
        "earned-chart",
        summary
            .earned
            .iter()
            .map(|row| {
                (
                    row.year,
                    [row.realized, row.dividends, row.interest, row.total],
                )
            })
            .collect(),
        |row| row.0.to_string().into(),
        |row| row.1[3],
        move |row, _, cx| {
            chart_tooltip(
                so_far(row.0.to_string(), row.0 == current_year),
                ["home.realized_profit", "nav.dividends", "imports.interest"]
                    .into_iter()
                    .zip(row.1)
                    .map(|(label, amount)| (t!(label).into(), money(amount)))
                    .collect(),
                Some((t!("home.total").into(), pln(row.1[3]))),
                cx,
            )
            .into_any_element()
        },
    )
    .height(px(196.));
    card(
        t!("home.by_year"),
        Some(t!("home.by_year_caption").into()),
        None,
        cx,
    )
    .child(chart)
    .child(
        div().mt(px(12.)).child(
            table.header(
                [
                    "field.year",
                    "home.realized_pln",
                    "home.dividends_pln",
                    "home.interest_pln",
                    "home.total_pln",
                ]
                .into_iter()
                .map(|key| t!(key).into())
                .collect(),
                cx,
            ),
        ),
    )
    .children(summary.earned.iter().rev().map(|row| {
        let year = row.year;
        let context = context.clone();
        table.year_row(
            year,
            [row.realized, row.dividends, row.interest, row.total]
                .into_iter()
                .map(money)
                .collect(),
            year == current_year,
            move |_, _, cx| {
                if !disabled {
                    context.select_year(Some(year), cx);
                }
            },
            cx,
        )
    }))
    .child(
        table.total_row(
            t!("year.all"),
            [
                summary.realized,
                summary.dividends,
                summary.interest,
                summary.earned.iter().map(|row| row.total).sum(),
            ]
            .into_iter()
            .map(money)
            .collect(),
            cx,
        ),
    )
    .child(
        notice::info(t!("home.by_year_note"), cx)
            .p(px(16.))
            .border_t_1()
            .border_color(palette(cx).line),
    )
}

fn holdings(
    summary: &PortfolioSummary,
    cost: Decimal,
    context: &PageContext,
    disabled: bool,
    cx: &App,
) -> Div {
    let stocks = context.clone();
    let largest = summary
        .positions
        .first()
        .map_or(Decimal::ZERO, |position| position.cost);
    card(
        t!("stocks.holdings"),
        Some(t!("home.holdings_caption").into()),
        Some(
            link("portfolio-stocks", t!("nav.stocks"), disabled, cx)
                .on_click(move |_, _, cx| stocks.navigate(Page::Stocks, cx)),
        ),
        cx,
    )
    .child(
        holding_frame(cx)
            .h(px(32.))
            .text_size(px(12.))
            .text_color(palette(cx).faint)
            .child(div().flex_1().min_w_0().child(t!("home.position")))
            .child(numeric_cell(110.).child(t!("column.shares")))
            .child(numeric_cell(150.).child(t!("lots.cost")))
            .child(
                div()
                    .w(px(150.))
                    .flex_shrink_0()
                    .child(t!("home.share_of_cost")),
            ),
    )
    .children(
        summary
            .positions
            .iter()
            .take(6)
            .enumerate()
            .map(|(index, position)| position_row(index, position, summary, cost, largest, cx)),
    )
    .when(summary.positions.len() > 6, |card| {
        let context = context.clone();
        let remaining = &summary.positions[6..];
        let label: SharedString = format!(
            "{} · {}",
            plural("home.more_positions", remaining.len() as u64),
            remaining
                .iter()
                .map(|position| {
                    // A ticker held in two providers would read as a duplicate.
                    let providers = summary
                        .positions
                        .iter()
                        .filter(|other| other.ticker == position.ticker)
                        .count();
                    if providers > 1 {
                        format!("{} ({})", position.ticker, position.provider)
                    } else {
                        position.ticker.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", "),
        )
        .into();
        let p = *palette(cx);
        card.child(
            gpui_kit::base::Button::new("more-positions")
                .role(Role::Button)
                .accessibility_label(label.clone())
                .w_full()
                .text_color(p.muted)
                .hover(move |style| style.bg(p.hover))
                .focus(move |style| style.bg(p.hover))
                .child(
                    holding_frame(cx)
                        .h(px(38.))
                        .text_size(px(12.))
                        .child(div().flex_1().min_w_0().truncate().child(label))
                        .child(Icon::new(IconName::ChevronRight).size(px(13.))),
                )
                .on_click(move |_, _, cx| {
                    if !disabled {
                        context.navigate(Page::Stocks, cx);
                    }
                }),
        )
    })
    .child(currencies(summary, cost, cx))
}

fn holding_frame(cx: &App) -> Div {
    h_flex()
        .w_full()
        .px(px(16.))
        .gap(px(12.))
        .border_t_1()
        .border_color(palette(cx).line)
}

fn numeric_cell(width: f32) -> Div {
    div()
        .w(px(width))
        .flex_shrink_0()
        .text_right()
        .font_features(tabular_digits())
}

fn position_row(
    index: usize,
    position: &Position,
    summary: &PortfolioSummary,
    cost: Decimal,
    largest: Decimal,
    cx: &App,
) -> Div {
    let p = palette(cx);
    let exchange = summary.listings.get(&position.ticker, &position.provider);
    let share_count = shares(position.shares);
    let amount = pln(position.cost);
    holding_frame(cx)
        .h(px(38.))
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .gap(px(8.))
                .child(
                    div()
                        .w(px(48.))
                        .flex_shrink_0()
                        .font_semibold()
                        .child(position.ticker.clone()),
                )
                .when_some(exchange, |row, exchange| {
                    row.child(value::chip(exchange.name.clone(), true, cx))
                })
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(p.muted)
                        .child(position.provider.clone()),
                ),
        )
        .child(numeric_cell(110.).child(
            value::full_tooltip(
                div().id(("holding-shares", index)).child(value::text(
                    &share_count,
                    true,
                    false,
                    cx,
                )),
                &share_count,
            ),
        ))
        .child(
            numeric_cell(150.).child(value::full_tooltip(
                div()
                    .id(("holding-cost", index))
                    .child(value::text(&amount, true, false, cx)),
                &amount,
            )),
        )
        .child(
            share_bar(
                ("holding-share", index),
                position.cost,
                largest,
                cost,
                t!("home.cost_percentage").into(),
                cx,
            )
            .w(px(150.))
            .flex_shrink_0(),
        )
}

fn currencies(summary: &PortfolioSummary, cost: Decimal, cx: &App) -> Div {
    let p = palette(cx);
    v_flex()
        .px(px(16.))
        .pt(px(14.))
        .pb(px(16.))
        .gap(px(10.))
        .border_t_1()
        .border_color(p.line)
        .child(
            h_flex()
                .flex_wrap()
                .gap(px(16.))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.))
                        .font_medium()
                        .text_color(p.muted)
                        .child(t!("home.by_currency")),
                )
                .children(summary.cost_by_currency.iter().enumerate().map(
                    |(index, (currency, cost))| {
                        let amount = pln(*cost);
                        h_flex()
                            .gap(px(5.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .font_features(tabular_digits())
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded(px(2.))
                                    .bg(p.currency_series(index)),
                            )
                            .child(currency.to_string())
                            .child(value::full_tooltip(
                                div()
                                    .id(("currency-cost", index))
                                    .text_color(p.text)
                                    .child(value::text(&amount, false, false, cx)),
                                &amount,
                            ))
                            .child("PLN")
                    },
                )),
        )
        .child(
            h_flex()
                .h(px(10.))
                .w_full()
                .rounded(px(4.))
                .overflow_hidden()
                .children(summary.cost_by_currency.iter().enumerate().map(
                    |(index, (currency, amount))| {
                        share_tooltip(
                            div()
                                .id(("currency-share", index))
                                .h_full()
                                .flex_shrink_0()
                                .w(relative(
                                    fraction(*amount, cost).to_f32().unwrap_or_default(),
                                ))
                                .bg(p.currency_series(index))
                                .when(index == 0, |bar| bar.rounded_l(px(4.)))
                                .when(index + 1 == summary.cost_by_currency.len(), |bar| {
                                    bar.rounded_r(px(4.))
                                }),
                            Some(currency.to_string().into()),
                            *amount,
                            cost,
                            t!("home.cost_percentage").into(),
                        )
                    },
                )),
        )
}

fn crypto(summary: &PortfolioSummary, context: &PageContext, disabled: bool, cx: &App) -> Div {
    let context = context.clone();
    card(
        t!("nav.crypto"),
        Some(
            format!(
                "{} · {}",
                and_list(&summary.crypto_providers),
                record_count(summary.crypto_records as u32)
            )
            .into(),
        ),
        Some(
            link("portfolio-crypto", t!("home.records"), disabled, cx)
                .on_click(move |_, _, cx| context.navigate(Page::Crypto, cx)),
        ),
        cx,
    )
    .children(
        [
            (
                "crypto-put-in",
                format!("{} {}", t!("crypto.put_in"), t!("crypto.buys_and_fees")),
                summary.crypto.put_in,
                false,
            ),
            (
                "crypto-taken-out",
                format!("{} {}", t!("crypto.taken_out"), t!("crypto.sales")),
                summary.crypto.taken_out,
                false,
            ),
            (
                "crypto-net",
                t!("dividends.net").into_owned(),
                summary.crypto.net,
                true,
            ),
        ]
        .into_iter()
        .map(|(id, label, amount, muted)| {
            let amount = pln(amount);
            h_flex()
                .h(px(46.))
                .px(px(16.))
                .gap(px(12.))
                .border_t_1()
                .border_color(palette(cx).line)
                .when(muted, |row| row.text_color(palette(cx).muted))
                .child(div().flex_1().child(label))
                .child(value::full_tooltip(
                    div()
                        .id(id)
                        .text_size(px(16.))
                        .font_features(tabular_digits())
                        .when(!muted, |value| value.font_medium())
                        .child(value::text(&amount, false, true, cx)),
                    &amount,
                ))
        }),
    )
}

pub fn skeleton(cx: &App) -> [Div; 2] {
    let block = |width, height| {
        div()
            .w(px(width))
            .h(px(height))
            .rounded(px(4.))
            .bg(cx.theme().skeleton)
    };
    [
        data::card(cx).child(h_flex().items_stretch().children(
            TOTAL_LABELS.into_iter().enumerate().map(|(index, label)| {
                total_cell(index, label, cx)
                    .child(block(120., 23.))
                    .child(block(140., 14.))
            }),
        )),
        card(
            t!("home.by_year"),
            Some(t!("home.by_year_caption").into()),
            None,
            cx,
        )
        .child(
            div()
                .h(px(232.))
                .m(px(16.))
                .rounded(px(4.))
                .bg(cx.theme().skeleton),
        )
        .children((0..3).map(|_| {
            div()
                .px(px(16.))
                .py(px(12.))
                .border_t_1()
                .border_color(palette(cx).line)
                .child(block(120., 16.))
        })),
    ]
}
