use super::{copy::CopyButton, value};
use crate::{
    format::{DATE_TIME_FORMAT, DisplayText, date},
    theme::{palette, tabular_digits},
};
use chrono::{DateTime, NaiveDate, Utc};
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Icon, StyledExt, VirtualListScrollHandle,
        button::{Button, ButtonVariants},
        h_flex,
        tag::Tag,
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use pitpls_core::time;
use rust_i18n::t;
use std::rc::Rc;

pub const PADDING: f32 = 12.;
/// Checkbox, chevron and "⋯" columns.
const FRAME: [f32; 3] = [30., 20., 36.];
const COMPACT_FRAME: [f32; 3] = [28., 18., 28.];
const GROW_MIN: f32 = 140.;
const COMPACT_GROW_MIN: f32 = 60.;
const NUMBER_INSET: f32 = 8.;
const RATE_GAP: f32 = 5.;
const RATE_DATE_TEXT: Pixels = px(12.);
pub const ROW_TEXT: Pixels = px(13.);
const STACK_BELOW: f32 = 680.;
/// Room under the last row, so the selection bar doesn't cover it.
pub const END_SPACE: Pixels = px(72.);

#[derive(Clone, Copy, PartialEq)]
pub enum CellStyle {
    Muted,
    /// `extra` holds the time of day, left out when compact.
    Time,
    Strong,
    Tag,
    Number,
    // Right-aligned so it doesn't run into the price's currency before it. Unlike
    // Time it keeps its date (`extra`) when compact: a rate means little without it.
    Rate,
}

/// `label` is a message key.
pub struct RecordColumn {
    pub label: &'static str,
    pub style: CellStyle,
    pub width: f32,
    pub compact: f32,
    pub grow: bool,
    pub optional: bool,
}

impl RecordColumn {
    pub const fn new(label: &'static str, style: CellStyle, width: f32, compact: f32) -> Self {
        Self {
            label,
            style,
            width,
            compact,
            grow: false,
            optional: false,
        }
    }

    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub const fn grow(label: &'static str, style: CellStyle) -> Self {
        Self {
            label,
            style,
            width: 0.,
            compact: 0.,
            grow: true,
            optional: false,
        }
    }
}

/// The table fills the viewport. When the normal widths don't fit, it takes the
/// compact ones, then leaves out the optional columns. Only when even that
/// doesn't fit does it get wider and scroll sideways.
#[derive(Clone, PartialEq)]
pub struct TableLayout {
    pub width: Pixels,
    pub frame: [Pixels; 3],
    pub columns: Vec<Option<Pixels>>,
    pub compact: bool,
    pub stacked: bool,
}

impl TableLayout {
    pub fn new(
        columns: &[RecordColumn],
        fit: &[Pixels],
        viewport: Pixels,
        selectable: bool,
    ) -> Self {
        let viewport = viewport / px(1.);
        let frames = |frame: [f32; 3]| {
            if selectable {
                frame
            } else {
                [0., frame[1], frame[2]]
            }
        };
        let widths = |compact: bool, optional: bool| -> Vec<Option<f32>> {
            columns
                .iter()
                .enumerate()
                .map(|(index, column)| {
                    if column.optional && !optional {
                        None
                    } else if column.grow {
                        Some(0.)
                    } else {
                        let width = if compact {
                            column.compact
                        } else {
                            column.width
                        };
                        let fit = fit.get(index).map_or(0., |fit| *fit / px(1.));
                        Some(width.max(fit))
                    }
                })
                .collect()
        };
        let used = |frame: [f32; 3], widths: &[Option<f32>]| {
            2. * PADDING + frame.iter().sum::<f32>() + widths.iter().flatten().sum::<f32>()
        };
        let (normal, small) = (frames(FRAME), frames(COMPACT_FRAME));
        let mut shown = widths(false, true);
        let compact = used(normal, &shown) + GROW_MIN > viewport;
        let (frame, grow_min) = if compact {
            shown = widths(true, true);
            if used(small, &shown) + COMPACT_GROW_MIN > viewport {
                shown = widths(true, false);
            }
            (small, COMPACT_GROW_MIN)
        } else {
            (normal, GROW_MIN)
        };
        let used = used(frame, &shown);
        let grow = (viewport - used).max(grow_min);
        Self {
            width: px(used + grow),
            frame: frame.map(px),
            columns: columns
                .iter()
                .zip(shown)
                .map(|(column, width)| {
                    width.map(|width| px(if column.grow { grow } else { width }))
                })
                .collect(),
            compact,
            stacked: viewport < STACK_BELOW,
        }
    }

    pub fn shown<T>(
        &self,
        items: impl IntoIterator<Item = T>,
    ) -> impl Iterator<Item = (T, Pixels)> {
        items
            .into_iter()
            .zip(&self.columns)
            .filter_map(|(item, width)| Some((item, (*width)?)))
    }

    pub fn indent(&self) -> Pixels {
        if self.stacked {
            px(46.)
        } else {
            px(PADDING) + self.frame[0] + self.frame[1]
        }
    }
}

pub fn row(layout: &TableLayout) -> Div {
    h_flex().w(layout.width).flex_shrink_0().px(px(PADDING))
}

/// Inside the row, since the table's scroll container clips the kit's ring,
/// which sits outside.
pub fn focus_ring(cx: &App) -> Div {
    div()
        .absolute()
        .inset_0()
        .border_2()
        .border_color(cx.theme().ring)
}

pub fn control(width: Pixels) -> Div {
    div()
        .w(width)
        .flex_shrink_0()
        .flex()
        .items_center()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

pub fn heading(column: &RecordColumn, width: Pixels) -> Div {
    div()
        .w(width)
        .flex_shrink_0()
        .truncate()
        .when(
            matches!(column.style, CellStyle::Number | CellStyle::Rate),
            |cell| cell.pl(px(NUMBER_INSET)).text_right(),
        )
        .child(t!(column.label))
}

/// Plain text: hundreds of selectable cells are too slow.
pub fn cell(
    id: impl Into<ElementId>,
    value: &DisplayText,
    column: &RecordColumn,
    width: Pixels,
    compact: bool,
    flagged: bool,
    cx: &App,
) -> AnyElement {
    let p = *palette(cx);
    let frame = div().w(width).flex_shrink_0().min_w_0().truncate();
    if flagged {
        return frame
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(12.))
            .text_color(p.warning)
            .child(
                Icon::new(IconName::TriangleAlert)
                    .size(px(12.))
                    .flex_shrink_0(),
            )
            .child(value.main.clone())
            .into_any_element();
    }
    match column.style {
        CellStyle::Muted => frame
            .text_color(p.muted)
            .font_features(tabular_digits())
            .child(value.main.clone())
            .into_any_element(),
        CellStyle::Time => frame
            .flex()
            .items_baseline()
            .gap(px(7.))
            .text_color(p.muted)
            .font_features(tabular_digits())
            .child(value.main.clone())
            .when(!compact && !value.extra.is_empty(), |cell| {
                cell.child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(value.extra.clone()),
                )
            })
            .into_any_element(),
        CellStyle::Strong => frame
            .font_semibold()
            .child(value.main.clone())
            .into_any_element(),
        CellStyle::Tag => frame
            .flex()
            .items_center()
            .child(
                Tag::custom(transparent_black(), p.muted, p.strong_line)
                    .rounded_full()
                    .h(px(20.))
                    .px(px(8.))
                    .py_0()
                    .text_size(px(12.))
                    .child(value.main.clone()),
            )
            .when(!value.extra.is_empty(), |cell| {
                cell.child(
                    div()
                        .ml(px(6.))
                        .min_w_0()
                        .truncate()
                        .text_color(p.muted)
                        .child(value.extra.clone()),
                )
            })
            .into_any_element(),
        CellStyle::Number => value::reveal_full(
            frame
                .id(id)
                .pl(px(NUMBER_INSET))
                .text_right()
                .font_features(tabular_digits())
                .child(value::text(value, true, true, cx)),
            value,
        ),
        CellStyle::Rate => frame
            .flex()
            .justify_end()
            .items_baseline()
            .gap(px(RATE_GAP))
            .pl(px(NUMBER_INSET))
            .font_features(tabular_digits())
            .child(value.main.clone())
            .when(!value.extra.is_empty(), |cell| {
                cell.child(
                    div()
                        .text_size(RATE_DATE_TEXT)
                        .text_color(p.faint)
                        .child(value.extra.clone()),
                )
            })
            .into_any_element(),
    }
}

pub fn plain_cell(
    id: impl Into<ElementId>,
    number: &DisplayText,
    column: &RecordColumn,
    width: Pixels,
    compact: bool,
    cx: &App,
) -> AnyElement {
    if !matches!(column.style, CellStyle::Number) {
        return cell(id, number, column, width, compact, false, cx);
    }
    value::full_tooltip(
        div()
            .id(id)
            .w(width)
            .flex_shrink_0()
            .min_w_0()
            .truncate()
            .pl(px(NUMBER_INSET))
            .text_right()
            .font_features(tabular_digits())
            .child(value::text(number, true, true, cx)),
        number,
    )
    .into_any_element()
}

/// With tabular digits the longest value in each unit is the widest, so only
/// those are measured.
pub fn fit_widths(
    columns: &[RecordColumn],
    rows: &[RowDisplay],
    window: &Window,
    cx: &App,
) -> Vec<Pixels> {
    columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            if column.style == CellStyle::Rate {
                return rate_width(
                    rows.iter().filter_map(|row| row.cells.get(index)),
                    window,
                    cx,
                );
            }
            if column.style != CellStyle::Number {
                return px(0.);
            }
            let mut longest: Vec<&DisplayText> = Vec::new();
            for value in rows.iter().filter_map(|row| row.cells.get(index)) {
                match longest.iter_mut().find(|known| known.unit == value.unit) {
                    Some(known) if value.main.chars().count() > known.main.chars().count() => {
                        *known = value
                    }
                    Some(_) => {}
                    None => longest.push(value),
                }
            }
            let widest = longest
                .into_iter()
                .map(|value| value::text_width(value, true, true, ROW_TEXT, window, cx))
                .fold(px(0.), Pixels::max);
            (px(NUMBER_INSET) + widest).ceil()
        })
        .collect()
}

fn rate_width<'a>(
    values: impl Iterator<Item = &'a DisplayText>,
    window: &Window,
    cx: &App,
) -> Pixels {
    let Some(widest) =
        values.max_by_key(|value| value.main.chars().count() + value.extra.chars().count())
    else {
        return px(0.);
    };
    let width = |text: &SharedString, size| {
        value::text_width(
            &DisplayText::plain(text.clone()),
            false,
            false,
            size,
            window,
            cx,
        )
    };
    let mut total = px(NUMBER_INSET) + width(&widest.main, ROW_TEXT);
    if !widest.extra.is_empty() {
        total += px(RATE_GAP) + width(&widest.extra, RATE_DATE_TEXT);
    }
    total.ceil()
}

#[derive(Clone, Copy, PartialEq)]
pub enum EntryStyle {
    Normal,
    Strong,
    Warning,
}

#[derive(PartialEq)]
pub enum StepLine {
    Formula(Vec<DisplayText>),
    Result(Vec<DisplayText>),
    Caption(SharedString),
    ValueCaption {
        label: SharedString,
        value: DisplayText,
        caption: SharedString,
    },
    Entry {
        label: SharedString,
        value: Vec<DisplayText>,
        style: EntryStyle,
    },
}

/// `title` is a message key.
#[derive(PartialEq)]
pub struct Step {
    pub title: &'static str,
    pub lines: Vec<StepLine>,
}

#[derive(PartialEq)]
pub struct LotTable {
    pub title: SharedString,
    pub caption: SharedString,
    pub rows: Vec<LotRow>,
    pub link: Option<SharedString>,
}

/// Cells in `LOT_COLUMNS` order; the NBP rate's `extra` holds its date.
#[derive(PartialEq)]
pub struct LotRow {
    pub cells: Vec<DisplayText>,
    pub uncovered: bool,
    pub notes: Vec<SharedString>,
}

#[derive(Default, PartialEq)]
pub struct RowDisplay {
    pub id: SharedString,
    pub time: DateTime<Utc>,
    pub cells: Vec<DisplayText>,
    pub steps: Vec<Step>,
    pub lots: Option<LotTable>,
    pub flagged: Vec<usize>,
    /// Lower case, for the filter.
    pub search: String,
    pub label: SharedString,
    pub links: Vec<SharedString>,
}

fn parts(id: impl Into<ElementId>, parts: &[DisplayText], copy: bool, cx: &App) -> Stateful<Div> {
    h_flex()
        .id(id)
        .flex_wrap()
        .gap_x(px(4.))
        .children(parts.iter().enumerate().map(|(index, part)| {
            value::reveal(
                div().id(index).child(value::text(part, false, true, cx)),
                part,
                copy,
            )
        }))
}

fn step(index: usize, step: &Step, copy: bool, cx: &App) -> Stateful<Div> {
    let p = *palette(cx);
    v_flex()
        .id(("step", index))
        .min_w_0()
        .gap(px(3.))
        .pt(px(10.))
        .pb(px(8.))
        .child(
            h_flex()
                .gap(px(8.))
                .mb(px(5.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(
                    div()
                        .size(px(18.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .border_1()
                        .border_color(p.strong_line)
                        .text_size(px(11.))
                        .child((index + 1).to_string()),
                )
                .child(t!(step.title)),
        )
        .children(step.lines.iter().enumerate().map(|(index, line)| {
            let id = ("line", index);
            match line {
                StepLine::Formula(formula) => parts(id, formula, copy, cx)
                    .text_color(p.muted)
                    .font_features(tabular_digits())
                    .into_any_element(),
                StepLine::Result(result) => parts(id, result, copy, cx)
                    .text_size(px(16.))
                    .font_medium()
                    .font_features(tabular_digits())
                    .into_any_element(),
                StepLine::Caption(text) => div()
                    .mt(px(3.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(text.clone())
                    .into_any_element(),
                StepLine::ValueCaption {
                    label,
                    value,
                    caption,
                } => h_flex()
                    .mt(px(3.))
                    .flex_wrap()
                    .gap_x(px(4.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(label.clone())
                    .child(value::full_tooltip(
                        div().id(id).child(value::text(value, false, true, cx)),
                        value,
                    ))
                    .child(div().min_w_0().max_w_full().child(caption.clone()))
                    .into_any_element(),
                StepLine::Entry {
                    label,
                    value,
                    style,
                } => h_flex()
                    .justify_between()
                    .items_baseline()
                    .gap(px(12.))
                    .when(!copy, |entry| entry.flex_wrap().gap_y_0())
                    .line_height(px(22.))
                    .when(*style == EntryStyle::Strong, |entry| entry.font_semibold())
                    .child(
                        div()
                            .min_w_0()
                            .when(!copy, |label| label.flex_shrink_0().max_w_full())
                            .text_color(match style {
                                EntryStyle::Normal => p.muted,
                                EntryStyle::Strong => p.text,
                                EntryStyle::Warning => p.warning,
                            })
                            .child(label.clone()),
                    )
                    .child(
                        parts(id, value, copy, cx)
                            .flex_shrink_0()
                            .when(!copy, |value| value.max_w_full().flex_grow(1.))
                            .justify_end()
                            .font_features(tabular_digits()),
                    )
                    .into_any_element(),
            }
        }))
}

/// The result labels are message keys.
pub struct Preview {
    pub nbp_date: NaiveDate,
    pub formula: Vec<DisplayText>,
    pub value: DisplayText,
    pub results: Vec<(&'static str, DisplayText)>,
}

pub fn preview_frame(note: Option<SharedString>, cx: &App) -> Div {
    let p = *palette(cx);
    v_flex()
        .gap(px(6.))
        .px(px(14.))
        .py(px(12.))
        .rounded(px(10.))
        .bg(p.hover)
        .border_1()
        .border_color(p.line)
        .font_features(tabular_digits())
        .child(
            h_flex()
                .justify_between()
                .gap_2()
                .text_size(px(12.))
                .text_color(p.faint)
                .child(t!("records.preview"))
                .children(note),
        )
}

pub fn preview_error(error: SharedString, cx: &App) -> Div {
    preview_frame(None, cx).child(div().text_color(palette(cx).warning).child(error))
}

pub fn preview_band(preview: &Result<Preview, SharedString>, copy: bool, cx: &App) -> Div {
    let p = *palette(cx);
    match preview {
        Ok(preview) => preview_frame(
            Some(t!("records.nbp_date", date = date(preview.nbp_date).main).into()),
            cx,
        )
        .child(
            h_flex()
                .flex_wrap()
                .gap_x(px(4.))
                .text_color(p.muted)
                .children(preview.formula.iter().enumerate().map(|(index, part)| {
                    preview_value(("preview-formula", index), part, div(), copy, cx)
                }))
                .child("=")
                .child(preview_value(
                    "preview-value",
                    &preview.value,
                    div().font_semibold().text_color(p.text),
                    copy,
                    cx,
                )),
        )
        .when(!preview.results.is_empty(), |band| {
            band.child(
                h_flex()
                    .flex_wrap()
                    .gap_x(px(20.))
                    .gap_y(px(4.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .children(preview.results.iter().map(|(label, result)| {
                        h_flex().gap(px(4.)).child(t!(*label)).child(preview_value(
                            *label,
                            result,
                            div().text_color(p.text),
                            copy,
                            cx,
                        ))
                    })),
            )
        }),
        Err(error) => preview_error(error.clone(), cx),
    }
}

pub fn preview_value(
    id: impl Into<ElementId>,
    display: &DisplayText,
    style: Div,
    copy: bool,
    cx: &App,
) -> AnyElement {
    value::reveal(
        style.id(id).child(value::text(display, false, true, cx)),
        display,
        copy,
    )
}

#[derive(Clone, Copy, PartialEq)]
enum LotCell {
    Muted,
    Shares,
    Taken,
    Number,
    Rate,
}

/// `label` is a message key.
struct LotColumn {
    label: &'static str,
    cell: LotCell,
    width: f32,
    min: f32,
    hide_compact: bool,
    hide_stacked: bool,
}

const fn lot_column(label: &'static str, cell: LotCell, width: f32, min: f32) -> LotColumn {
    LotColumn {
        label,
        cell,
        width,
        min,
        hide_compact: false,
        hide_stacked: false,
    }
}

const LOT_COLUMNS: [LotColumn; 9] = [
    lot_column("lots.bought", LotCell::Muted, 80., 72.),
    LotColumn {
        hide_stacked: true,
        ..lot_column("lots.lot", LotCell::Shares, 64., 44.)
    },
    LotColumn {
        hide_compact: true,
        ..lot_column("lots.before", LotCell::Shares, 64., 44.)
    },
    lot_column("lots.taken", LotCell::Taken, 64., 44.),
    LotColumn {
        hide_compact: true,
        ..lot_column("lots.after", LotCell::Shares, 64., 44.)
    },
    LotColumn {
        hide_stacked: true,
        ..lot_column("lots.price", LotCell::Number, 112., 80.)
    },
    LotColumn {
        hide_stacked: true,
        ..lot_column("lots.nbp_rate", LotCell::Rate, 112., 88.)
    },
    // Takes the rest of the row.
    lot_column("lots.cost", LotCell::Number, 0., 88.),
    lot_column("lots.buy_fee", LotCell::Number, 96., 72.),
];

const LOT_GAP: f32 = 8.;

fn lot_row() -> Div {
    h_flex().gap(px(LOT_GAP)).px(px(PADDING))
}

fn lot_columns(layout: &TableLayout) -> impl Iterator<Item = (usize, &'static LotColumn)> {
    let (compact, stacked) = (layout.compact, layout.stacked);
    LOT_COLUMNS.iter().enumerate().filter(move |(_, column)| {
        !(column.hide_compact && compact || column.hide_stacked && stacked)
    })
}

fn lot_slot(column: &LotColumn) -> Div {
    let slot = div().min_w(px(column.min)).truncate();
    let slot = if column.width == 0. {
        slot.flex_1()
    } else {
        slot.w(px(column.width)).flex_shrink(1.)
    };
    if column.cell == LotCell::Muted {
        slot
    } else {
        slot.text_right()
    }
}

pub fn link(id: impl Into<ElementId>, label: SharedString, cx: &App) -> Button {
    Button::new(id)
        .ghost()
        .h(px(26.))
        .pl(px(8.))
        .pr(px(6.))
        .gap(px(4.))
        .rounded(px(7.))
        .text_color(palette(cx).muted)
        .accessibility_label(label.clone())
        .child(div().text_size(px(12.)).child(label))
        .child(Icon::new(IconName::ChevronRight).size(px(13.)))
}

fn lot_table(
    table: &LotTable,
    layout: &TableLayout,
    link: Option<AnyElement>,
    copy: bool,
    cx: &App,
) -> Div {
    let p = *palette(cx);
    let headings = lot_row()
        .h(px(28.))
        .border_t_1()
        .border_b_1()
        .border_color(p.line)
        .text_size(px(12.))
        .text_color(p.faint)
        .children(lot_columns(layout).map(|(_, column)| lot_slot(column).child(t!(column.label))));
    let rows = table.rows.iter().enumerate().map(|(row_index, row)| {
        let cells = lot_columns(layout).map(|(index, column)| {
            let value = &row.cells[index];
            let slot = lot_slot(column).font_features(tabular_digits());
            let id = SharedString::from(format!("lot-{row_index}-{index}"));
            match column.cell {
                LotCell::Muted if row.uncovered => slot
                    .text_color(p.warning)
                    .font_medium()
                    .child(value.main.clone())
                    .into_any_element(),
                LotCell::Muted => slot
                    .text_color(p.muted)
                    .child(value.main.clone())
                    .into_any_element(),
                LotCell::Rate => slot
                    .child(value.main.clone())
                    .when(!value.extra.is_empty(), |slot| {
                        slot.child(
                            div()
                                .ml(px(5.))
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(value.extra.clone()),
                        )
                    })
                    .flex()
                    .justify_end()
                    .items_baseline()
                    .into_any_element(),
                LotCell::Shares | LotCell::Taken | LotCell::Number => value::reveal(
                    slot.id(id)
                        .when(column.cell == LotCell::Shares, |slot| {
                            slot.text_color(p.muted)
                        })
                        .when(column.cell == LotCell::Taken, |slot| slot.font_semibold())
                        .child(value::text(value, true, true, cx)),
                    value,
                    copy,
                ),
            }
        });
        v_flex()
            .id(("lot-row", row_index))
            .when(row_index > 0, |lot| lot.border_t_1().border_color(p.line))
            .when(row.uncovered, |lot| lot.bg(p.warning.opacity(0.08)))
            .child(lot_row().h(px(34.)).children(cells))
            .children(row.notes.iter().map(|note| {
                h_flex()
                    .gap(px(6.))
                    .mt(px(-4.))
                    .pb(px(8.))
                    .pl(px(PADDING + LOT_COLUMNS[0].width + LOT_GAP))
                    .pr(px(PADDING))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(
                        Icon::new(IconName::CornerDownRight)
                            .size(px(12.))
                            .flex_shrink_0(),
                    )
                    .child(div().min_w_0().child(note.clone()))
            }))
    });
    v_flex()
        .mt(px(10.))
        .rounded(px(10.))
        .border_1()
        .border_color(p.line)
        .bg(p.surface)
        .overflow_hidden()
        .text_size(ROW_TEXT)
        .child(
            h_flex()
                .min_h(px(36.))
                .py(px(6.))
                .px(px(PADDING))
                .gap(px(8.))
                .flex_wrap()
                .text_size(px(12.))
                .text_color(p.faint)
                .child(
                    div()
                        .text_color(p.muted)
                        .font_medium()
                        .child(table.title.clone()),
                )
                .child(table.caption.clone())
                .when_some(link, |header, link| {
                    header.child(div().flex_1()).child(link)
                }),
        )
        .child(headings)
        .children(rows)
}

/// The host builds the links, since they call back into it.
pub fn details(
    row: &RowDisplay,
    layout: &TableLayout,
    links: Vec<AnyElement>,
    lots_link: Option<AnyElement>,
    actions: impl IntoIterator<Item = AnyElement>,
    copy: bool,
    cx: &App,
) -> Div {
    let p = *palette(cx);
    let (record_id, time, steps) = (row.id.as_ref(), row.time, &row.steps);
    let indent = layout.indent();
    let steps = if layout.stacked {
        v_flex()
            .ml(indent)
            .mr(px(PADDING))
            .children(steps.iter().enumerate().map(|(index, item)| {
                step(index, item, copy, cx)
                    .when(index > 0, |step| step.border_t_1().border_color(p.line))
            }))
    } else {
        // Stretched columns, so the dividers run the full height.
        div()
            .flex()
            .ml(indent)
            .mr(px(PADDING))
            .children(steps.iter().enumerate().map(|(index, item)| {
                step(index, item, copy, cx)
                    .flex_1()
                    .pr(px(20.))
                    .when(index > 0, |step| {
                        step.pl(px(20.)).border_l_1().border_color(p.line)
                    })
            }))
            .children((steps.len()..3).map(|_| div().flex_1().min_w_0()))
    };
    v_flex()
        .w(layout.width)
        .pt(px(4.))
        .pb(px(10.))
        .bg(p.raised)
        .border_b_1()
        .border_color(p.line)
        .child(steps)
        .when_some(row.lots.as_ref(), |details, lots| {
            details.child(
                div()
                    .ml(indent)
                    .mr(px(PADDING))
                    .child(lot_table(lots, layout, lots_link, copy, cx)),
            )
        })
        .child(
            h_flex()
                .mt(px(if row.lots.is_some() { 10. } else { 8. }))
                .ml(indent)
                .mr(px(PADDING))
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.faint)
                .child(div().flex_shrink_0().child(t!("records.record_id")))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.5))
                        .font_family(cx.theme().mono_font_family.clone())
                        .child(SharedString::from(record_id.to_owned())),
                )
                .when(copy, |footer| {
                    footer.child(CopyButton::new(
                        SharedString::from(format!("copy-id-{record_id}")),
                        record_id.to_owned(),
                    ))
                })
                .child(record_time(time, layout.stacked, cx))
                .when(!links.is_empty(), |footer| {
                    footer.child(h_flex().ml(px(6.)).gap(px(2.)).children(links))
                })
                .child(div().flex_1())
                .children(actions),
        )
}

fn record_time(value: DateTime<Utc>, short: bool, cx: &App) -> Div {
    let local = time::local(value);
    let utc = value.naive_utc();
    let utc_format = if utc.date() == local.date() {
        "%H:%M:%S"
    } else {
        DATE_TIME_FORMAT
    };
    h_flex()
        .flex_shrink_0()
        .ml(px(14.))
        .gap(px(6.))
        .font_features(tabular_digits())
        .child(t!("records.time"))
        .child(
            div()
                .text_color(palette(cx).muted)
                .child(local.format(DATE_TIME_FORMAT).to_string()),
        )
        .child(if short {
            t!("records.warsaw")
        } else {
            t!("records.warsaw_utc", time = utc.format(utc_format))
        })
}

pub fn record_skeleton(
    columns: &[RecordColumn],
    layout: &TableLayout,
    visible: bool,
    cx: &App,
) -> Div {
    let p = *palette(cx);
    let bar = |width: Pixels| {
        div()
            .h(px(12.))
            .w(width * 0.6)
            .rounded(px(4.))
            .bg(cx.theme().skeleton)
            .opacity(if visible { 1. } else { 0. })
    };
    v_flex()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .flex_1()
        .min_h(rems(17.))
        .child(
            row(layout)
                .h(px(32.))
                .border_t_1()
                .border_b_1()
                .border_color(p.line)
                .text_size(px(12.))
                .text_color(p.faint)
                .child(div().w(layout.frame[0] + layout.frame[1]).flex_shrink_0())
                .children(
                    layout
                        .shown(columns)
                        .map(|(column, width)| heading(column, width)),
                ),
        )
        .children((0..5).map(|_| {
            row(layout)
                .h(px(40.))
                .border_b_1()
                .border_color(p.line)
                .child(div().w(layout.frame[0] + layout.frame[1]).flex_shrink_0())
                .children(
                    layout
                        .shown(columns)
                        .map(|(_, width)| div().w(width).flex_shrink_0().child(bar(width))),
                )
        }))
}

#[derive(Clone, Copy, PartialEq)]
pub enum Item {
    Month(usize),
    Record(usize),
    End,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ItemKey {
    Month(i32, u32),
    Record(SharedString),
    End,
}

pub struct RecordTableState {
    pub rows: Vec<RowDisplay>,
    pub items: Vec<Item>,
    keys: Vec<ItemKey>,
    pub sizes: Rc<Vec<Size<Pixels>>>,
    /// Shared by every collapsed row: their cells are single truncated lines.
    pub collapsed: Option<Size<Pixels>>,
    pub month: Option<Size<Pixels>>,
    pub expanded: Vec<Option<Size<Pixels>>>,
    pub scroll: VirtualListScrollHandle,
    pub viewport_width: Option<Pixels>,
    pub fit: Option<Vec<Pixels>>,
    pub layout: Option<TableLayout>,
    pub layout_key: Option<(TableLayout, Pixels, SharedString, SharedString)>,
    pub dirty: bool,
}

impl Default for RecordTableState {
    fn default() -> Self {
        Self {
            rows: vec![],
            items: vec![],
            keys: vec![],
            sizes: Rc::new(vec![]),
            collapsed: None,
            month: None,
            expanded: vec![],
            scroll: VirtualListScrollHandle::new(),
            viewport_width: None,
            fit: None,
            layout: None,
            layout_key: None,
            dirty: true,
        }
    }
}

impl RecordTableState {
    /// The items stay until the next rebuild, so the first visible one keeps
    /// its place if it's still listed.
    pub fn reset(&mut self, rows: Vec<RowDisplay>) {
        self.rows = rows;
        self.fit = None;
        self.invalidate_measurements();
    }

    pub fn invalidate_measurements(&mut self) {
        self.collapsed = None;
        self.month = None;
        self.expanded = vec![None; self.rows.len()];
        self.dirty = true;
    }

    /// Also reveals the items next to it, so Tab and Shift-Tab find them
    /// rendered.
    pub fn reveal(&self, key: &ItemKey) {
        let Some(index) = self.keys.iter().position(|item| item == key) else {
            return;
        };
        let height = |index: usize| self.sizes.get(index).map_or(px(0.), |size| size.height);
        let top = self.sizes[..index]
            .iter()
            .map(|size| size.height)
            .sum::<Pixels>();
        let start = top - index.checked_sub(1).map_or(px(0.), height);
        let end = top + height(index) + height(index + 1);
        let viewport = self.scroll.bounds().size.height;
        let mut offset = self.scroll.offset();
        if start < -offset.y {
            offset.y = -start;
        } else if end > viewport - offset.y {
            offset.y = viewport - end;
        }
        // The list clamps it to its content.
        self.scroll.set_offset(offset);
    }

    /// Clearing the keys skips anchoring on the next rebuild.
    pub fn scroll_to_top(&mut self) {
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.keys.clear();
        self.sizes = Rc::new(vec![]);
        self.dirty = true;
    }

    pub fn set_items(&mut self, items: Vec<Item>, keys: Vec<ItemKey>, sizes: Vec<Size<Pixels>>) {
        // Keep the first visible item in place when items above it change
        // height or fold away. If it is gone, keep the nearest one above it.
        let mut top = px(0.);
        let mut offset = self.scroll.offset();
        let first = self.sizes.iter().position(|item| {
            top += item.height;
            top > -offset.y
        });
        let anchor = first.and_then(|first| {
            (0..=first).rev().find_map(|old| {
                let new = keys.iter().position(|key| *key == self.keys[old])?;
                Some((old, new))
            })
        });
        if let Some((old, new)) = anchor {
            let old_top = self.sizes[..old].iter().map(|s| s.height).sum::<Pixels>();
            let new_top = sizes[..new].iter().map(|s| s.height).sum::<Pixels>();
            offset.y -= new_top - old_top;
            self.scroll.set_offset(offset);
        }
        self.items = items;
        self.keys = keys;
        self.sizes = Rc::new(sizes);
        self.dirty = false;
    }
}
