mod display;

use super::{
    display::InstrumentKey,
    exchange::Catalog,
    set_exchange::{ExchangeDialog, ExchangeDialogEvent},
};
use crate::{
    components::{
        ButtonText, Status, header, measure_width,
        records::{self, CellStyle, RecordColumn, TableLayout, cell, heading},
        spinner, value,
    },
    navigation::{Page, PageContext},
    theme::{Palette, palette, tabular_digits},
};
use display::{HistoryDisplay, LotBar, Shade, YearGroup};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{button::*, scroll::ScrollableElement, tooltip::Tooltip, *},
    *,
};
use pitpls_app::use_case::stock;
use rust_i18n::t;
use std::borrow::Cow;

pub enum HistoryEvent {
    Back,
    Changed,
}

pub struct HistoryView {
    context: PageContext,
    key: InstrumentKey,
    year: Option<i32>,
    status: Status,
    display: Option<HistoryDisplay>,
    catalog: Entity<Catalog>,
    dialog: Option<(Entity<ExchangeDialog>, Subscription)>,
    columns: Vec<RecordColumn>,
    fit: Option<Vec<Pixels>>,
    width: Option<Pixels>,
    pan: ScrollHandle,
    scroll: ScrollHandle,
    focus: FocusHandle,
}

impl EventEmitter<HistoryEvent> for HistoryView {}

impl HistoryView {
    pub fn new(
        context: PageContext,
        key: InstrumentKey,
        year: Option<i32>,
        catalog: Entity<Catalog>,
        width: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            context,
            key,
            year,
            status: Status::default(),
            display: None,
            catalog,
            dialog: None,
            columns: vec![
                RecordColumn::new("column.date", CellStyle::Time, 96., 50.),
                RecordColumn::grow("column.event", CellStyle::Tag),
                RecordColumn::new("column.shares", CellStyle::Number, 64., 56.),
                RecordColumn::new("column.price", CellStyle::Number, 112., 96.).optional(),
                RecordColumn::new("lots.nbp_rate", CellStyle::Rate, 112., 64.).optional(),
                RecordColumn::new("column.value_pln", CellStyle::Number, 120., 100.),
                RecordColumn::new("column.profit_pln", CellStyle::Number, 120., 100.),
                RecordColumn::new("column.holding_after", CellStyle::Number, 112., 96.),
            ],
            fit: None,
            width,
            pan: ScrollHandle::default(),
            scroll: ScrollHandle::default(),
            focus: cx.focus_handle(),
        };
        view.refresh(window, cx);
        view
    }

    pub fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    fn editable(&self) -> bool {
        self.display.is_some() && !self.status.loading && self.dialog.is_none()
    }

    fn edit_instrument(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(display) = &self.display else {
            return;
        };
        if !self.editable() {
            return;
        }
        let dialog = ExchangeDialog::open(
            self.context.services.clone(),
            self.catalog.clone(),
            t!("history.edit_instrument"),
            vec![display.instrument.clone()],
            window,
            cx,
        );
        let subscription =
            cx.subscribe_in(&dialog, window, |this, _, event, window, cx| match event {
                ExchangeDialogEvent::Saved(message) => {
                    this.close_dialog(cx);
                    this.refresh(window, cx);
                    this.status.message = Some(message.clone().into());
                    cx.emit(HistoryEvent::Changed);
                }
                ExchangeDialogEvent::Closed => this.close_dialog(cx),
            });
        self.dialog = Some((dialog, subscription));
        self.status.error = None;
        self.status.message = None;
        self.context.set_locked(true, cx);
        cx.notify();
    }

    fn close_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        self.context.set_locked(false, cx);
        cx.notify();
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status.begin_load(window, cx, |this| &mut this.status);
        let (key, year) = (self.key.clone(), self.year);
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let history = stock::load_history(&app, &key.ticker, &key.provider, year).await?;
                Ok(HistoryDisplay::new(&history, &key, year))
            },
            |this, result, _, cx| {
                if let Some(display) = this.status.loaded(result) {
                    this.display = Some(display);
                    this.fit = None;
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn facts(display: &HistoryDisplay, cx: &App) -> Div {
        let p = *palette(cx);
        h_flex()
            .flex_shrink_0()
            .flex_wrap()
            .gap_x(px(36.))
            .gap_y(px(8.))
            .px(px(20.))
            .pt(px(2.))
            .pb(px(14.))
            .children(display.facts.iter().map(|fact| {
                v_flex()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(fact.label.clone()),
                    )
                    .child(
                        div()
                            .font_features(tabular_digits())
                            .when(fact.warning, |value| value.text_color(p.warning))
                            .child(fact.value.clone()),
                    )
            }))
    }

    fn lots(&self, display: &HistoryDisplay, cx: &App) -> Div {
        let p = *palette(cx);
        let card = v_flex()
            .flex_shrink_0()
            .mx(px(20.))
            .mb(px(16.))
            .px(px(16.))
            .pt(px(6.))
            .pb(px(10.))
            .rounded(px(12.))
            .border_1()
            .border_color(p.line)
            .bg(p.raised);
        if display.lots.is_empty() {
            return card.child(div().py(px(8.)).text_color(p.muted).child(match self.year {
                Some(year) => t!("history.no_buys", year = year),
                None => t!("stocks.history_no_buys"),
            }));
        }
        card.children(
            display
                .lots
                .iter()
                .enumerate()
                .map(|(index, lot)| lot_row(index, lot, &p)),
        )
        .when(!display.legend.is_empty(), |card| {
            card.child(
                h_flex()
                    .flex_wrap()
                    .gap_x(px(16.))
                    .gap_y(px(4.))
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .children(display.legend.iter().map(|(shade, label)| {
                        h_flex()
                            .gap(px(6.))
                            .child(
                                shaded(div(), *shade, &p)
                                    .w(px(14.))
                                    .h(px(10.))
                                    .rounded(px(3.)),
                            )
                            .child(label.clone())
                    })),
            )
        })
    }

    fn timeline(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = *palette(cx);
        let Some(display) = &self.display else {
            return div().into_any_element();
        };
        // TODO: Plain rows, not a virtual list like the record tables. Fine for one
        // instrument's records; revisit if a long DCA history scrolls slowly.
        let fit = self
            .fit
            .get_or_insert_with(|| records::fit_widths(&self.columns, &display.rows, window, cx));
        let table = TableLayout::new(&self.columns, fit, self.width.unwrap_or(px(900.)), false);
        let spacer = || div().w(table.frame[0] + table.frame[1]).flex_shrink_0();
        let note_indent = px(records::PADDING)
            + table.frame[0]
            + table.frame[1]
            + table.columns[0].unwrap_or_default();

        let mut rows = v_flex().w(table.width).child(
            records::row(&table)
                .h(px(32.))
                .border_t_1()
                .border_b_1()
                .border_color(p.line)
                .text_size(px(12.))
                .text_color(p.faint)
                .child(spacer())
                .children(
                    table
                        .shown(&self.columns)
                        .map(|(column, width)| heading(column, width)),
                ),
        );
        for group in &display.years {
            rows = rows.child(year_row(group, &table, self.year.is_some(), cx));
            for index in group.rows.clone() {
                let row = &display.rows[index];
                let notes = &display.notes[index];
                let cells = table
                    .shown(row.cells.iter().zip(&self.columns).enumerate())
                    .map(|((column, (value, kind)), width)| {
                        let id = SharedString::from(format!("event-{index}-{column}"));
                        if self.year.is_some() {
                            cell(id, value, kind, width, table.compact, false, cx)
                        } else {
                            records::plain_cell(id, value, kind, width, table.compact, cx)
                        }
                    });
                rows = rows.child(
                    v_flex()
                        .border_b_1()
                        .border_color(p.line)
                        .child(
                            records::row(&table)
                                .h(px(38.))
                                .text_size(records::ROW_TEXT)
                                .child(spacer())
                                .children(cells),
                        )
                        .when(!notes.is_empty(), |event| {
                            event.child(
                                v_flex()
                                    .mt(px(-6.))
                                    .pb(px(8.))
                                    .pl(note_indent)
                                    .pr(px(records::PADDING) + table.frame[2])
                                    .gap(px(2.))
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .font_features(tabular_digits())
                                    .children(notes.iter().cloned()),
                            )
                        }),
                );
            }
        }

        div()
            .id("history-timeline")
            .w_full()
            .min_w_0()
            .relative()
            .flex_shrink_0()
            .overflow_hidden()
            .child(measure_width(
                self.width,
                |view: &mut Self| &mut view.width,
                cx,
            ))
            .child(
                div()
                    .id("timeline-pan")
                    .w_full()
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.pan)
                    .child(rows),
            )
            .horizontal_scrollbar(&self.pan)
            .into_any_element()
    }

    fn carried(&self, display: &HistoryDisplay, cx: &App) -> Option<Div> {
        let year = self.year?;
        let p = *palette(cx);
        (!display.carried.is_empty()).then(|| {
            h_flex()
                .flex_shrink_0()
                .items_start()
                .gap(px(10.))
                .mx(px(20.))
                .mt(px(12.))
                .px(px(14.))
                .py(px(10.))
                .rounded(px(10.))
                .border_1()
                .border_dashed()
                .border_color(p.strong_line)
                .child(
                    div()
                        .flex_shrink_0()
                        .font_medium()
                        .child(t!("history.into", year = year + 1)),
                )
                .child(
                    v_flex()
                        .min_w_0()
                        .text_color(p.muted)
                        .font_features(tabular_digits())
                        .children(display.carried.iter().cloned()),
                )
        })
    }
}

fn section(title: Cow<'static, str>, caption: SharedString, cx: &App) -> Div {
    h_flex()
        .flex_shrink_0()
        .items_baseline()
        .gap(px(8.))
        .px(px(20.))
        .pb(px(8.))
        .child(div().flex_shrink_0().font_semibold().child(title))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(12.))
                .text_color(palette(cx).faint)
                .child(caption),
        )
}

fn shaded<E: Styled>(element: E, shade: Shade, p: &Palette) -> E {
    match shade {
        Shade::First => element.bg(p.text.opacity(0.18)),
        Shade::Second => element.bg(p.text.opacity(0.1)),
        Shade::Open => element
            .border_1()
            .border_dashed()
            .border_color(p.strong_line),
    }
}

fn lot_row(index: usize, lot: &LotBar, p: &Palette) -> Div {
    let parts = lot.parts.iter().enumerate().map(|(part_index, part)| {
        let tooltip = part.tooltip.clone();
        shaded(
            div().id(SharedString::from(format!("lot-{index}-part-{part_index}"))),
            part.shade,
            p,
        )
        .flex_basis(px(0.))
        .flex_grow(part.weight)
        .min_w(px(3.))
        .h_full()
        .flex()
        .items_center()
        .rounded(px(6.))
        .overflow_hidden()
        .when(part.shade == Shade::Open, |part| part.text_color(p.muted))
        .child(
            div()
                .min_w_0()
                .px(px(8.))
                .truncate()
                .text_size(px(12.))
                .child(part.label.clone()),
        )
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
    });
    h_flex()
        .min_h(px(46.))
        .py(px(6.))
        .gap(px(16.))
        .font_features(tabular_digits())
        .when(index > 0, |row| row.border_t_1().border_color(p.line))
        .child(
            v_flex()
                .w(px(180.))
                .flex_shrink_0()
                .child(div().font_medium().child(lot.bought.clone()))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(lot.label.clone()),
                ),
        )
        .child(
            div().flex_1().min_w_0().h(px(26.)).child(
                h_flex()
                    .h_full()
                    .w(relative(lot.width))
                    .gap(px(2.))
                    .children(parts),
            ),
        )
        .child(
            div()
                .w(px(120.))
                .flex_shrink_0()
                .text_right()
                .text_color(p.muted)
                .child(lot.status.clone()),
        )
}

fn year_row(group: &YearGroup, table: &TableLayout, copy: bool, cx: &App) -> Div {
    let p = *palette(cx);
    records::row(table)
        .h(px(34.))
        .gap(px(10.))
        .border_b_1()
        .border_color(p.line)
        .bg(p.raised)
        .child(
            div()
                .w(table.frame[0] + table.frame[1])
                .flex_shrink_0()
                .mr(px(-10.)),
        )
        .child(
            div()
                .font_semibold()
                .font_features(tabular_digits())
                .child(group.year.to_string()),
        )
        .child(div().flex_1())
        .child(
            h_flex()
                .flex_shrink_0()
                .text_size(px(12.))
                .text_color(p.faint)
                .font_features(tabular_digits())
                .child(group.counts.clone())
                .when_some(group.profit.as_ref(), |summary, profit| {
                    let number = div()
                        .id(SharedString::from(format!("year-profit-{}", group.year)))
                        .child(value::text(profit, false, true, cx));
                    let separator = if group.counts.is_empty() { "" } else { " · " };
                    summary
                        .child(format!("{separator}{} ", t!("records.subtotal_profit")))
                        .child(if copy {
                            value::reveal_full(number, profit).into_any_element()
                        } else {
                            value::full_tooltip(number, profit).into_any_element()
                        })
                }),
        )
}

impl Render for HistoryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = *palette(cx);
        let mut body = v_flex()
            .id("history-body")
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .lock_scroll_axis()
            .track_scroll(&self.scroll)
            .when(self.status.is_visible(), |body| {
                body.child(
                    div()
                        .flex_shrink_0()
                        .px(px(20.))
                        .pb_3()
                        .child(self.status.render(|this| &mut this.status, cx)),
                )
            });
        let timeline = self.display.is_some().then(|| self.timeline(window, cx));
        if let (Some(display), Some(timeline)) = (&self.display, timeline) {
            body = body
                .child(Self::facts(display, cx))
                .child(section(
                    t!("history.lots"),
                    display.lots_caption.clone(),
                    cx,
                ))
                .child(self.lots(display, cx))
                .child(section(
                    t!("history.timeline"),
                    display.timeline_caption.clone(),
                    cx,
                ))
                .child(timeline)
                .children(self.carried(display, cx))
                .child(div().flex_shrink_0().h(px(24.)));
        } else if self.status.loading {
            body = body.child(
                h_flex()
                    .px(px(20.))
                    .py(px(12.))
                    .gap_2()
                    .text_color(p.muted)
                    .when(!self.status.loading_visible, |row| row.invisible())
                    .child(spinner())
                    .child(t!("history.loading", ticker = self.key.ticker)),
            );
        } else {
            body = body.child(
                h_flex().px(px(20.)).child(
                    Button::new("retry")
                        .text_label(t!("common.retry"))
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            );
        }

        let editable = self.editable();
        v_flex()
            .id("stock-history")
            .track_focus(&self.focus)
            .tab_index(-1)
            .size_full()
            .min_h_0()
            .child(
                header::breadcrumb(
                    Page::Stocks.title(),
                    self.key.ticker.clone(),
                    self.key.provider.clone(),
                    cx.listener(|_, _, _, cx| cx.emit(HistoryEvent::Back)),
                    window,
                    cx,
                )
                .child(
                    header::actions().child(
                        header::button("edit-instrument", None, t!("history.edit_instrument"))
                            .disabled(!editable)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.edit_instrument(window, cx)),
                            ),
                    ),
                ),
            )
            .child(
                v_flex()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(body)
                    .vertical_scrollbar(&self.scroll),
            )
    }
}
