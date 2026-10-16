use super::{
    StocksPage,
    display::{HoldingBlock, InstrumentKey, StockDisplay, lots_count, shares_count},
};
use crate::{
    components::{
        measure_width,
        records::{self, CellStyle, RecordColumn, TableLayout, cell, heading},
    },
    pages::records::TableHost,
    theme::{palette, tabular_digits},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{scroll::ScrollableElement, tag::Tag, *},
    *,
};
use rust_decimal::Decimal;

pub struct Layout {
    columns: Vec<RecordColumn>,
    pub fit: Option<Vec<Pixels>>,
    pub width: Option<Pixels>,
    scroll: ScrollHandle,
}

impl Layout {
    pub fn new(width: Option<Pixels>) -> Self {
        Self {
            columns: vec![
                RecordColumn::new("Bought", CellStyle::Time, 130., 84.),
                RecordColumn::new("Lot", CellStyle::Number, 64., 56.),
                RecordColumn::new("Open", CellStyle::Number, 64., 56.),
                RecordColumn::new("Price", CellStyle::Number, 112., 96.),
                RecordColumn::new("NBP rate", CellStyle::Rate, 112., 64.).optional(),
                RecordColumn::grow("Cost, PLN", CellStyle::Number),
                RecordColumn::new("Buy fee, PLN", CellStyle::Number, 104., 88.),
            ],
            fit: None,
            width,
            scroll: ScrollHandle::default(),
        }
    }
}

fn matches(block: &HoldingBlock, query: &str) -> bool {
    block.search.contains(query)
}

pub fn shown(display: &StockDisplay, query: &str) -> usize {
    display
        .holdings
        .iter()
        .filter(|block| matches(block, query))
        .count()
}

fn chip(text: SharedString, color: Hsla, cx: &App) -> Tag {
    Tag::custom(transparent_black(), color, palette(cx).strong_line)
        .rounded(px(6.))
        .h(px(20.))
        .px(px(7.))
        .py_0()
        .text_size(px(12.))
        .child(text)
}

impl StocksPage {
    pub fn holdings_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = *palette(cx);
        let year = self.year.unwrap_or_default();
        let disabled = self.disabled();
        let Some(display) = &self.display else {
            return div().into_any_element();
        };
        let layout = &mut self.holdings;
        if layout.fit.is_none() {
            let rows = display
                .holdings
                .iter()
                .flat_map(|block| block.lots.iter())
                .map(|row| records::RowDisplay {
                    cells: row.cells.clone(),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            layout.fit = Some(records::fit_widths(&layout.columns, &rows, window, cx));
        }
        let table = TableLayout::new(
            &layout.columns,
            layout.fit.as_deref().unwrap_or_default(),
            layout.width.unwrap_or(px(900.)),
            false,
        );
        let spacer = || div().w(table.frame[0] + table.frame[1]).flex_shrink_0();
        let blocks = display
            .holdings
            .iter()
            .filter(|block| matches(block, &self.query))
            .collect::<Vec<_>>();
        let sold_out = display
            .sold_out
            .iter()
            .filter(|sold| sold.search.contains(&self.query))
            .collect::<Vec<_>>();

        let headings = records::row(&table)
            .h(px(32.))
            .border_t_1()
            .border_b_1()
            .border_color(p.line)
            .text_size(px(12.))
            .text_color(p.faint)
            .child(spacer())
            .children(
                table
                    .shown(&layout.columns)
                    .map(|(column, width)| heading(column, width)),
            );

        let mut rows = v_flex().w(table.width).child(headings);
        for (block_index, block) in blocks.iter().enumerate() {
            let exchange = match &block.exchange {
                Some(exchange) => chip(exchange.clone(), p.muted, cx),
                None => chip("No exchange".into(), p.warning, cx),
            };
            let key = InstrumentKey {
                ticker: block.ticker.clone(),
                provider: block.provider.clone(),
            };
            let history = records::link(
                SharedString::from(format!("history-{block_index}")),
                "History".into(),
                cx,
            )
            .accessibility_label(format!("{} history in {}", block.ticker, block.provider))
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _, window, cx| this.open_history(key.clone(), window, cx)),
            );
            rows = rows.child(
                records::row(&table)
                    .h(px(40.))
                    .gap(px(10.))
                    .border_b_1()
                    .border_color(p.line)
                    .bg(p.raised)
                    .child(spacer().mr(px(-10.)))
                    .child(div().font_semibold().child(block.ticker.clone()))
                    .child(exchange)
                    .child(chip(block.provider.clone(), p.muted, cx))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .font_features(tabular_digits())
                            .child(format!(
                                "{} · {}",
                                shares_count(block.shares),
                                lots_count(block.lots.len())
                            )),
                    )
                    .child(history.mr(px(-6.))),
            );
            rows = rows.children(block.lots.iter().enumerate().map(|(lot_index, lot)| {
                records::row(&table)
                    .h(px(40.))
                    .text_size(records::ROW_TEXT)
                    .border_b_1()
                    .border_color(p.line)
                    .child(spacer())
                    .children(
                        table
                            .shown(lot.cells.iter().zip(&layout.columns).enumerate())
                            .map(|((column, (value, kind)), width)| {
                                cell(
                                    SharedString::from(format!(
                                        "holding-{block_index}-{lot_index}-{column}"
                                    )),
                                    value,
                                    kind,
                                    width,
                                    table.compact,
                                    false,
                                    cx,
                                )
                            }),
                    )
            }));
        }
        if blocks.is_empty() {
            let text = if display.holdings.is_empty() {
                format!("No open lots on 31.12.{year}.")
            } else {
                format!("No holdings match “{}”.", self.query)
            };
            rows = rows.child(
                div()
                    .px(px(20.))
                    .py(px(24.))
                    .text_color(p.muted)
                    .child(text),
            );
        }
        for (index, sold) in sold_out.iter().enumerate() {
            rows = rows.child(
                records::row(&table)
                    .h(px(40.))
                    .gap(px(10.))
                    .border_b_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(spacer().mr(px(-10.)))
                    .child(
                        div()
                            .flex_shrink_0()
                            .when(index > 0, |label| label.invisible())
                            .child(format!("Sold out in {year}:")),
                    )
                    .child(div().text_color(p.muted).child(sold.instrument.clone()))
                    .child(format!("· {}", sold.last_sale)),
            );
        }
        let open = blocks.iter().map(|block| block.shares).sum::<Decimal>();
        let lots = blocks.iter().map(|block| block.lots.len()).sum::<usize>();
        if lots > 0 {
            rows = rows.child(
                h_flex()
                    .mt(px(14.))
                    .mb(px(24.))
                    .ml(table.indent())
                    .mr(px(12.))
                    .gap(px(10.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(Icon::new(IconName::Info).size(px(14.)).flex_shrink_0())
                    .child(div().min_w_0().child(format!(
                        "{} in {} move into {}. FIFO takes from them first, oldest lot first, \
                         per provider.",
                        shares_count(open),
                        lots_count(lots),
                        year + 1
                    ))),
            );
        }

        div()
            .id("stock-holdings")
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
                    .id("holdings-pan")
                    .w_full()
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.holdings.scroll)
                    .child(rows),
            )
            .horizontal_scrollbar(&self.holdings.scroll)
            .into_any_element()
    }
}
