use super::{
    StocksPage, Tab,
    display::{InstrumentKey, rate_cell},
};
use crate::{
    components::{
        measure_width, notice, records,
        tabs::{self, TabItem},
        value,
    },
    format::{DisplayText, amount, date, pln, shares},
    i18n::{plural, plural_key},
    pages::records::{TableHost, totals_row},
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    assets::IconName,
    component::{scroll::ScrollableElement, tag::Tag, tooltip::Tooltip, *},
    prelude::FluentBuilder,
    *,
};
use pitpls_app::use_case::portfolio::StockPortfolio;
use pitpls_core::{common::Amount, time::tax_date};
use rust_decimal::Decimal;
use rust_i18n::t;
use std::collections::{BTreeMap, HashSet};

#[derive(Default)]
pub struct Layout {
    open: HashSet<(SharedString, SharedString)>,
    pan: ScrollHandle,
}

pub struct PositionRow {
    key: InstrumentKey,
    exchange: Option<SharedString>,
    search: String,
    shares: DisplayText,
    average: DisplayText,
    cost: Decimal,
    since: DisplayText,
    lots: Vec<[DisplayText; 5]>,
}

struct ClosedPosition {
    ticker: String,
    provider: String,
    search: String,
}

pub struct PortfolioDisplay {
    pub positions: Vec<PositionRow>,
    cost: DisplayText,
    realized: DisplayText,
    instruments: usize,
    sales: usize,
    first_sale: Option<i32>,
    closed: Vec<ClosedPosition>,
}

impl PortfolioDisplay {
    pub fn new(data: &StockPortfolio) -> Self {
        let positions = data
            .positions
            .iter()
            .map(|position| {
                let exchange = data
                    .listings
                    .get(&position.ticker, &position.provider)
                    .map(|listing| SharedString::from(listing.name.clone()));
                let mut lots = data
                    .replay
                    .open_lots
                    .iter()
                    .filter(|lot| {
                        lot.ticker == position.ticker
                            && lot.provider == position.provider
                            && lot.price.currency == position.currency
                    })
                    .collect::<Vec<_>>();
                lots.sort_by_key(|lot| (lot.bought, &lot.buy_id));
                PositionRow {
                    key: InstrumentKey {
                        ticker: position.ticker.clone().into(),
                        provider: position.provider.clone().into(),
                    },
                    search: format!(
                        "{} {} {}",
                        position.ticker,
                        position.provider,
                        exchange.as_deref().unwrap_or("")
                    )
                    .to_lowercase(),
                    exchange,
                    shares: shares(position.shares),
                    average: amount(Amount {
                        value: position.avg_price,
                        currency: position.currency,
                    }),
                    cost: position.cost,
                    since: date(tax_date(position.since)),
                    lots: lots
                        .into_iter()
                        .map(|lot| {
                            [
                                date(tax_date(lot.bought)),
                                shares(lot.open),
                                amount(lot.price),
                                rate_cell(lot.nbp_rate, lot.nbp_date, lot.price.currency),
                                pln(lot.cost + lot.fee_part),
                            ]
                        })
                        .collect(),
                }
            })
            .collect();
        let closed = data
            .realized_instruments
            .iter()
            .filter(|instrument| !instrument.held)
            .map(|instrument| {
                let exchange = data
                    .listings
                    .get(&instrument.ticker, &instrument.provider)
                    .map_or("", |listing| listing.name.as_str());
                ClosedPosition {
                    ticker: instrument.ticker.clone(),
                    provider: instrument.provider.clone(),
                    search: format!("{} {} {exchange}", instrument.ticker, instrument.provider)
                        .to_lowercase(),
                }
            })
            .collect();
        Self {
            positions,
            cost: pln(data.positions.iter().map(|position| position.cost).sum()),
            realized: pln(data.replay.sales.iter().map(|sale| sale.profit).sum()),
            instruments: data.realized_instruments.len(),
            sales: data.replay.sales.len(),
            first_sale: data.realized_years.first().map(|row| row.year),
            closed,
        }
    }

    pub fn shown(&self, query: &str) -> usize {
        self.positions
            .iter()
            .filter(|position| position.search.contains(query))
            .count()
    }

    pub fn stats(&self, compact: bool, cx: &App) -> Div {
        totals_row(compact)
            .child(value::plain_stat(
                "holdings-cost",
                t!("stocks.portfolio_cost"),
                &self.cost,
                Some("PLN".into()),
                None,
                cx,
            ))
            .child(value::plain_stat(
                "realized-profit",
                t!("stocks.realized_profit"),
                &self.realized,
                Some("PLN".into()),
                None,
                cx,
            ))
            .child(value::plain_stat(
                "positions",
                t!("stocks.positions"),
                &DisplayText::plain(self.positions.len().to_string()),
                None,
                Some(plural("stocks.open_closed", self.closed.len() as u64).into()),
                cx,
            ))
            .child(value::plain_stat(
                "sales",
                t!("stocks.sales"),
                &DisplayText::plain(self.sales.to_string()),
                None,
                self.first_sale
                    .map(|year| t!("stocks.since_year", year = year).into()),
                cx,
            ))
    }

    pub fn tabs(
        &self,
        selected: Tab,
        records: usize,
        select: impl Fn(&Tab, &mut Window, &mut App) + 'static,
        cx: &App,
    ) -> Div {
        let caption = match selected {
            Tab::Holdings => "stocks.portfolio_holdings_caption",
            Tab::Sales | Tab::Realized => "stocks.portfolio_realized_caption",
            Tab::Records => "stocks.portfolio_records_caption",
        };
        tabs::tabs(
            "stock-tabs",
            t!("stocks.view"),
            vec![
                TabItem {
                    key: Tab::Holdings,
                    label: t!("stocks.holdings").into(),
                    count: self.positions.len(),
                },
                TabItem {
                    key: Tab::Realized,
                    label: t!("stocks.realized").into(),
                    count: self.instruments,
                },
                TabItem {
                    key: Tab::Records,
                    label: t!("stocks.records").into(),
                    count: records,
                },
            ],
            selected,
            select,
            t!(caption),
            cx,
        )
    }
}

const WIDTHS: [f32; 9] = [28., 56., 76., 0., 100., 140., 136., 92., 44.];

fn row(cx: &App) -> Div {
    h_flex()
        .w_full()
        .px(px(12.))
        .gap(px(8.))
        .border_b_1()
        .border_color(palette(cx).line)
        .font_features(tabular_digits())
}

fn slot(index: usize) -> Div {
    div().min_w_0().when_else(
        index == 3,
        |cell| cell.flex_1(),
        |cell| cell.w(px(WIDTHS[index])).flex_shrink_0(),
    )
}

fn number(id: impl Into<ElementId>, amount: &DisplayText, unit: bool, cx: &App) -> Stateful<Div> {
    value::full_tooltip(
        div()
            .id(id)
            .text_right()
            .font_features(tabular_digits())
            .child(value::text(amount, true, unit, cx)),
        amount,
    )
}

impl StocksPage {
    pub fn portfolio_holdings(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(portfolio) = self
            .display
            .as_ref()
            .and_then(|display| display.portfolio.as_ref())
        else {
            return div().into_any_element();
        };
        let p = *palette(cx);
        let disabled = self.disabled();
        let width = self.holdings.width.unwrap_or(px(940.)).max(px(940.));
        let positions = portfolio
            .positions
            .iter()
            .filter(|position| position.search.contains(&self.query))
            .collect::<Vec<_>>();
        let mut rows = v_flex().w(width).text_size(px(13.)).child(
            row(cx)
                .h(px(32.))
                .border_t_1()
                .border_color(p.line)
                .text_size(px(12.))
                .text_color(p.faint)
                .children(
                    [
                        None,
                        Some("column.ticker"),
                        Some("column.exchange"),
                        Some("column.provider"),
                        Some("column.shares"),
                        Some("stocks.average_price"),
                        Some("lots.cost"),
                        Some("stocks.since"),
                        Some("history.lots"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, label)| {
                        slot(index)
                            .when(matches!(index, 4..=6 | 8), |cell| cell.text_right())
                            .children(label.map(|label| t!(label)))
                    }),
                ),
        );
        for (index, position) in positions.iter().enumerate() {
            let key = (position.key.ticker.clone(), position.key.provider.clone());
            let expanded = self.portfolio.open.contains(&key);
            let label = t!(
                "stocks.toggle_position",
                ticker = position.key.ticker,
                provider = position.key.provider
            );
            rows = rows.child(
                gpui_kit::base::Button::new(("position", index))
                    .role(Role::Button)
                    .accessibility_label(label)
                    .aria_expanded(expanded)
                    .w_full()
                    .text_color(p.text)
                    .when(expanded, |row| row.bg(p.raised))
                    .hover(move |style| style.bg(p.hover))
                    .focus(move |style| style.bg(p.hover))
                    .child(
                        row(cx)
                            .h(px(40.))
                            .child(
                                slot(0).text_color(p.faint).child(
                                    Icon::new(if expanded {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    })
                                    .size(px(14.)),
                                ),
                            )
                            .child(slot(1).font_semibold().child(position.key.ticker.clone()))
                            .child(slot(2).children(position.exchange.clone().map(|exchange| {
                                Tag::custom(transparent_black(), p.muted, p.strong_line)
                                    .h(px(20.))
                                    .px(px(7.))
                                    .py_0()
                                    .rounded(px(6.))
                                    .text_size(px(12.))
                                    .child(exchange)
                            })))
                            .child(
                                slot(3)
                                    .truncate()
                                    .text_color(p.muted)
                                    .child(position.key.provider.clone()),
                            )
                            .child(slot(4).child(number(
                                ("position-shares", index),
                                &position.shares,
                                false,
                                cx,
                            )))
                            .child(slot(5).child(number(
                                ("position-average", index),
                                &position.average,
                                true,
                                cx,
                            )))
                            .child(slot(6).child(number(
                                ("position-cost", index),
                                &pln(position.cost),
                                false,
                                cx,
                            )))
                            .child(
                                slot(7)
                                    .text_color(p.muted)
                                    .child(position.since.main.clone()),
                            )
                            .child(
                                slot(8)
                                    .text_right()
                                    .text_color(p.muted)
                                    .child(position.lots.len().to_string()),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.disabled() {
                            return;
                        }
                        if !this.portfolio.open.remove(&key) {
                            this.portfolio.open.insert(key.clone());
                        }
                        cx.notify();
                    })),
            );
            if expanded {
                rows = rows.child(lots(position, index, cx));
                let key = position.key.clone();
                let caption = t!(
                    plural_key("stocks.open_lots_note", position.lots.len() as u64),
                    count = position.lots.len(),
                    ticker = key.ticker,
                    provider = key.provider
                );
                rows = rows.child(
                    row(cx)
                        .min_h(px(34.))
                        .pl(px(48.))
                        .bg(p.raised)
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(div().flex_1().min_w_0().child(caption))
                        .child(
                            records::link(
                                ("position-history", index),
                                t!("stocks.history").into(),
                                cx,
                            )
                            .accessibility_label(t!(
                                "stocks.history_label",
                                ticker = key.ticker,
                                provider = key.provider
                            ))
                            .disabled(disabled)
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open_history(key.clone(), window, cx)
                                },
                            )),
                        ),
                );
            }
        }
        if positions.is_empty() {
            rows = rows.child(div().px(px(48.)).py(px(24.)).text_color(p.muted).child(
                if portfolio.positions.is_empty() {
                    t!("stocks.no_open_lots_today")
                } else {
                    t!("stocks.no_holdings_match", query = self.query)
                },
            ));
        }
        let cost = positions.iter().map(|position| position.cost).sum();
        let lots = positions
            .iter()
            .map(|position| position.lots.len())
            .sum::<usize>();
        rows = rows.child(
            row(cx)
                .h(px(40.))
                .font_semibold()
                .child(slot(0))
                .child(div().flex_1().child(if self.query.is_empty() {
                    t!("stocks.all_positions")
                } else {
                    t!("stocks.filtered_positions")
                }))
                .child(slot(6).child(number("positions-cost", &pln(cost), false, cx)))
                .child(slot(7))
                .child(slot(8).text_right().child(lots.to_string())),
        );
        let mut closed: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for position in portfolio
            .closed
            .iter()
            .filter(|position| position.search.contains(&self.query))
        {
            closed
                .entry(&position.provider)
                .or_default()
                .push(&position.ticker);
        }
        if !closed.is_empty() {
            let groups = closed
                .into_iter()
                .map(|(provider, mut tickers)| {
                    tickers.sort_unstable();
                    format!("{} · {provider}", tickers.join(" · "))
                })
                .collect::<Vec<_>>()
                .join(", ");
            rows = rows.child(
                row(cx)
                    .min_h(px(40.))
                    .pl(px(48.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(div().flex_shrink_0().child(t!("stocks.sold_out_all")))
                    .child(div().flex_1().min_w_0().text_color(p.muted).child(groups))
                    .child(
                        records::link("holdings-realized", t!("stocks.realized").into(), cx)
                            .disabled(disabled)
                            .on_click(
                                cx.listener(|this, _, _, cx| this.select_tab(Tab::Realized, cx)),
                            ),
                    ),
            );
        }
        rows = rows.child(
            notice::info(t!("stocks.holdings_note"), cx)
                .ml(px(48.))
                .mr(px(12.))
                .mt(px(14.))
                .mb(px(24.)),
        );
        div()
            .id("portfolio-holdings")
            .w_full()
            .min_w_0()
            .relative()
            .flex_shrink_0()
            .overflow_hidden()
            .child(measure_width(
                self.holdings.width,
                |page: &mut Self| &mut page.holdings.width,
                cx,
            ))
            .child(
                div()
                    .id("portfolio-holdings-pan")
                    .w_full()
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.portfolio.pan)
                    .child(rows),
            )
            .horizontal_scrollbar(&self.portfolio.pan)
            .into_any_element()
    }
}

fn lots(position: &PositionRow, index: usize, cx: &App) -> Div {
    let p = palette(cx);
    let columns = [
        records::RecordColumn::new("lots.bought", records::CellStyle::Time, 110., 110.),
        records::RecordColumn::new("stocks.open_shares", records::CellStyle::Number, 120., 120.),
        records::RecordColumn::new("lots.price", records::CellStyle::Number, 130., 130.),
        records::RecordColumn::new("lots.nbp_rate", records::CellStyle::Rate, 90., 90.),
    ];
    v_flex()
        .bg(p.raised)
        .child(
            row(cx)
                .h(px(30.))
                .pl(px(48.))
                .text_size(px(12.))
                .text_color(p.faint)
                .children(
                    columns
                        .iter()
                        .map(|column| records::heading(column, px(column.width))),
                )
                .child(div().flex_1().text_right().child(t!("lots.cost")))
                .child(div().w(px(120.)).flex_shrink_0()),
        )
        .children(position.lots.iter().enumerate().map(|(lot_index, lot)| {
            let cost = &lot[4];
            let tooltip: SharedString = t!("stocks.lot_cost_tooltip", cost = cost.full).into();
            row(cx)
                .h(px(34.))
                .pl(px(48.))
                .text_size(px(12.5))
                .children(columns.iter().zip(lot).enumerate().map(
                    |(column_index, (column, amount))| {
                        records::plain_cell(
                            SharedString::from(format!(
                                "open-lot-{index}-{lot_index}-{column_index}"
                            )),
                            amount,
                            column,
                            px(column.width),
                            true,
                            cx,
                        )
                    },
                ))
                .child(
                    div().flex_1().child(
                        div()
                            .id(SharedString::from(format!(
                                "open-lot-cost-{index}-{lot_index}"
                            )))
                            .text_right()
                            .child(value::text(cost, true, false, cx))
                            .tooltip(move |window, cx| {
                                Tooltip::new(tooltip.clone()).build(window, cx)
                            }),
                    ),
                )
                .child(div().w(px(120.)).flex_shrink_0())
        }))
}
