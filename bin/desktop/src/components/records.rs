use super::{copy::CopyButton, value};
use crate::{
    format::{DisplayText, date},
    theme::{palette, tabular_digits},
};
use chrono::NaiveDate;
use gpui_kit::{
    component::{ActiveTheme, StyledExt, VirtualListScrollHandle, h_flex, tag::Tag, v_flex},
    prelude::FluentBuilder,
    *,
};
use std::rc::Rc;

const PADDING: f32 = 12.;
/// Checkbox, chevron and "⋯" columns.
const FRAME: [f32; 3] = [30., 20., 36.];
const COMPACT_FRAME: [f32; 3] = [28., 18., 28.];
const GROW_MIN: f32 = 140.;
const COMPACT_GROW_MIN: f32 = 60.;
const NUMBER_INSET: f32 = 8.;
pub const ROW_TEXT: Pixels = px(13.);
const STACK_BELOW: f32 = 680.;
/// Room under the last row, so the selection bar doesn't cover it.
pub const END_SPACE: Pixels = px(72.);

#[derive(Clone, Copy, PartialEq)]
pub enum CellStyle {
    Muted,
    Strong,
    Tag,
    Number,
}

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
    pub fn new(columns: &[RecordColumn], fit: &[Pixels], viewport: Pixels) -> Self {
        let viewport = viewport / px(1.);
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
        let mut shown = widths(false, true);
        let compact = used(FRAME, &shown) + GROW_MIN > viewport;
        let (frame, grow_min) = if compact {
            shown = widths(true, true);
            if used(COMPACT_FRAME, &shown) + COMPACT_GROW_MIN > viewport {
                shown = widths(true, false);
            }
            (COMPACT_FRAME, COMPACT_GROW_MIN)
        } else {
            (FRAME, GROW_MIN)
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
        .when(column.style == CellStyle::Number, |cell| {
            cell.pl(px(NUMBER_INSET)).text_right()
        })
        .child(column.label)
}

/// Plain text: hundreds of selectable cells are too slow.
pub fn cell(
    id: impl Into<ElementId>,
    value: &DisplayText,
    column: &RecordColumn,
    width: Pixels,
    cx: &App,
) -> AnyElement {
    let p = *palette(cx);
    let frame = div().w(width).flex_shrink_0().min_w_0().truncate();
    match column.style {
        CellStyle::Muted => frame
            .text_color(p.muted)
            .font_features(tabular_digits())
            .child(value.main.clone())
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
    }
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

#[derive(PartialEq)]
pub enum StepLine {
    Formula(Vec<DisplayText>),
    Result(Vec<DisplayText>),
    Caption(SharedString),
    Entry {
        label: &'static str,
        value: DisplayText,
        strong: bool,
    },
}

#[derive(PartialEq)]
pub struct Step {
    pub title: &'static str,
    pub lines: Vec<StepLine>,
}

#[derive(PartialEq)]
pub struct RowDisplay {
    pub cells: Vec<DisplayText>,
    pub steps: Vec<Step>,
    /// Lower case, for the filter.
    pub search: String,
    pub label: SharedString,
}

fn parts(id: impl Into<ElementId>, parts: &[DisplayText], cx: &App) -> Stateful<Div> {
    h_flex()
        .id(id)
        .flex_wrap()
        .gap_x(px(4.))
        .children(parts.iter().enumerate().map(|(index, part)| {
            value::reveal_full(
                div().id(index).child(value::text(part, false, true, cx)),
                part,
            )
        }))
}

fn step(index: usize, step: &Step, cx: &App) -> Stateful<Div> {
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
                .child(step.title),
        )
        .children(step.lines.iter().enumerate().map(|(index, line)| {
            let id = ("line", index);
            match line {
                StepLine::Formula(formula) => parts(id, formula, cx)
                    .text_color(p.muted)
                    .font_features(tabular_digits())
                    .into_any_element(),
                StepLine::Result(result) => parts(id, result, cx)
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
                StepLine::Entry {
                    label,
                    value,
                    strong,
                } => h_flex()
                    .justify_between()
                    .items_baseline()
                    .gap(px(12.))
                    .line_height(px(22.))
                    .when(*strong, |entry| entry.font_semibold())
                    .child(
                        div()
                            .text_color(if *strong { p.text } else { p.muted })
                            .child(*label),
                    )
                    .child(value::reveal_full(
                        div()
                            .id(id)
                            .text_right()
                            .font_features(tabular_digits())
                            .child(value::text(value, false, true, cx)),
                        value,
                    ))
                    .into_any_element(),
            }
        }))
}

pub struct Preview {
    pub nbp_date: NaiveDate,
    pub formula: Vec<DisplayText>,
    pub value: DisplayText,
    pub results: Vec<(&'static str, DisplayText)>,
}

pub fn preview_band(preview: &Result<Preview, SharedString>, cx: &App) -> Div {
    let p = *palette(cx);
    let header = h_flex()
        .justify_between()
        .gap_2()
        .text_size(px(12.))
        .text_color(p.faint)
        .child("Preview");
    let band = v_flex()
        .gap(px(6.))
        .px(px(14.))
        .py(px(12.))
        .rounded(px(10.))
        .bg(p.hover)
        .border_1()
        .border_color(p.line)
        .font_features(tabular_digits());
    match preview {
        Ok(preview) => band
            .child(header.child(format!("NBP date {}", date(preview.nbp_date).main)))
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_x(px(4.))
                    .text_color(p.muted)
                    .children(preview.formula.iter().enumerate().map(|(index, part)| {
                        preview_value(("preview-formula", index), part, div(), cx)
                    }))
                    .child("=")
                    .child(preview_value(
                        "preview-value",
                        &preview.value,
                        div().font_semibold().text_color(p.text),
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
                            h_flex().gap(px(4.)).child(*label).child(preview_value(
                                *label,
                                result,
                                div().text_color(p.text),
                                cx,
                            ))
                        })),
                )
            }),
        Err(error) => band
            .child(header)
            .child(div().text_color(p.warning).child(error.clone())),
    }
}

fn preview_value(
    id: impl Into<ElementId>,
    display: &DisplayText,
    style: Div,
    cx: &App,
) -> AnyElement {
    value::reveal_full(
        style.id(id).child(value::text(display, false, true, cx)),
        display,
    )
}

pub fn details(
    record_id: &str,
    steps: &[Step],
    layout: &TableLayout,
    actions: impl IntoIterator<Item = AnyElement>,
    cx: &App,
) -> Div {
    let p = *palette(cx);
    let indent = layout.indent();
    let steps = if layout.stacked {
        v_flex()
            .ml(indent)
            .mr(px(PADDING))
            .children(steps.iter().enumerate().map(|(index, item)| {
                step(index, item, cx).when(index > 0, |step| step.border_t_1().border_color(p.line))
            }))
    } else {
        // Stretched columns, so the dividers run the full height.
        div()
            .flex()
            .ml(indent)
            .mr(px(PADDING))
            .children(steps.iter().enumerate().map(|(index, item)| {
                step(index, item, cx)
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
        .child(
            h_flex()
                .mt(px(8.))
                .ml(indent)
                .mr(px(PADDING))
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.faint)
                .child(div().flex_shrink_0().child("Record ID"))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.5))
                        .font_family(cx.theme().mono_font_family.clone())
                        .child(SharedString::from(record_id.to_owned())),
                )
                .child(CopyButton::new(
                    SharedString::from(format!("copy-id-{record_id}")),
                    record_id.to_owned(),
                ))
                .child(div().flex_1())
                .children(actions),
        )
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

/// Measures the stationary viewport, not the sideways-scrolling table, and
/// reports it after layout.
pub fn measure_width<V: 'static>(
    current: Option<Pixels>,
    state: fn(&mut V) -> &mut RecordTableState,
    cx: &Context<V>,
) -> impl IntoElement {
    let view = cx.entity().downgrade();
    canvas(
        move |bounds, _, cx| {
            let width = bounds.size.width;
            if width > px(0.) && current != Some(width) {
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| {
                        state(view).viewport_width = Some(width);
                        cx.notify();
                    });
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
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
