use super::{
    copy::CopyButton,
    table::{self, Column},
};
use crate::{format::DisplayText, theme::palette, theme::tabular_digits};
use gpui_kit::{
    component::{ActiveTheme, StyledExt, VirtualListScrollHandle, h_flex, tag::Tag, v_flex},
    prelude::FluentBuilder,
    *,
};
use std::rc::Rc;

/// Side padding of every table row.
const PADDING: f32 = 12.;
/// Widths of the checkbox, chevron and "⋯" columns, normal and compact.
const FRAME: [f32; 3] = [30., 20., 36.];
const COMPACT_FRAME: [f32; 3] = [28., 18., 28.];
/// The least width the growing column keeps, normal and compact.
const GROW_MIN: f32 = 140.;
const COMPACT_GROW_MIN: f32 = 60.;
/// Below this table width, an opened row stacks its steps.
const STACK_BELOW: f32 = 680.;
/// Room under the last row, so the selection bar doesn't cover it.
pub const END_SPACE: Pixels = px(72.);

/// How a record column draws its cells.
#[derive(Clone, Copy, PartialEq)]
pub enum CellStyle {
    /// Dates and secondary text.
    Muted,
    /// The record's name, such as a ticker.
    Strong,
    /// A small outlined pill, such as Buy or Sell.
    Tag,
    /// A right-aligned number with a fixed slot for its extra decimals.
    Number,
}

/// A column of a record table, with px widths at the normal and the minimum
/// window size. The column that grows takes whatever width is left.
pub struct RecordColumn {
    pub label: &'static str,
    pub style: CellStyle,
    pub width: f32,
    pub compact: f32,
    pub grow: bool,
}

impl RecordColumn {
    pub const fn new(label: &'static str, style: CellStyle, width: f32, compact: f32) -> Self {
        Self {
            label,
            style,
            width,
            compact,
            grow: false,
        }
    }

    pub const fn grow(label: &'static str, style: CellStyle) -> Self {
        Self {
            label,
            style,
            width: 0.,
            compact: 0.,
            grow: true,
        }
    }
}

/// Column widths for a table viewport. The table fills the viewport and only
/// gets wider, scrolling sideways, when even the compact widths don't fit.
#[derive(Clone, PartialEq)]
pub struct TableLayout {
    pub width: Pixels,
    /// The checkbox, chevron and "⋯" columns.
    pub frame: [Pixels; 3],
    pub columns: Vec<Pixels>,
    pub compact: bool,
    /// Whether an opened row stacks its steps.
    pub stacked: bool,
}

impl TableLayout {
    pub fn new(columns: &[RecordColumn], viewport: Pixels) -> Self {
        let viewport = viewport / px(1.);
        let fixed = |frame: [f32; 3], width: fn(&RecordColumn) -> f32| {
            2. * PADDING
                + frame.iter().sum::<f32>()
                + columns
                    .iter()
                    .filter(|column| !column.grow)
                    .map(width)
                    .sum::<f32>()
        };
        let normal = fixed(FRAME, |column| column.width);
        let compact = normal + GROW_MIN > viewport;
        let (frame, used, grow_min) = if compact {
            (
                COMPACT_FRAME,
                fixed(COMPACT_FRAME, |column| column.compact),
                COMPACT_GROW_MIN,
            )
        } else {
            (FRAME, normal, GROW_MIN)
        };
        let grow = (viewport - used).max(grow_min);
        Self {
            width: px(used + grow),
            frame: frame.map(px),
            columns: columns
                .iter()
                .map(|column| {
                    px(if column.grow {
                        grow
                    } else if compact {
                        column.compact
                    } else {
                        column.width
                    })
                })
                .collect(),
            compact,
            stacked: viewport < STACK_BELOW,
        }
    }

    /// Where an opened row's content starts: under the first data column, or
    /// close to the edge once the steps stack.
    pub fn indent(&self) -> Pixels {
        if self.stacked {
            px(46.)
        } else {
            px(PADDING) + self.frame[0] + self.frame[1]
        }
    }
}

/// A table row's horizontal frame: full table width and the side padding.
pub fn row(layout: &TableLayout) -> Div {
    h_flex().w(layout.width).flex_shrink_0().px(px(PADDING))
}

/// Keeps a press on a control inside a clickable row from also opening or
/// folding the row.
pub fn control(width: Pixels) -> Div {
    div()
        .w(width)
        .flex_shrink_0()
        .flex()
        .items_center()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

/// A header cell: the column's label, right-aligned over numbers.
pub fn heading(column: &RecordColumn, width: Pixels) -> Div {
    div()
        .w(width)
        .flex_shrink_0()
        .truncate()
        .when(column.style == CellStyle::Number, |cell| {
            cell.pl(px(8.)).text_right()
        })
        .child(column.label)
}

/// A record cell as plain text: hundreds of selectable cells are too slow.
/// Numbers that hide digits show them in a tooltip and the context menu.
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
        CellStyle::Number => super::value::reveal_full(
            frame
                .id(id)
                .pl(px(8.))
                .text_right()
                .font_features(tabular_digits())
                .child(super::value::text(value, true, true, cx)),
            value,
        ),
    }
}

/// A line of an opened row's step.
#[derive(PartialEq)]
pub enum StepLine {
    /// The calculation, such as `8.352 USD × 3.6142`.
    Formula(SharedString),
    /// The step's result, with every digit.
    Result(SharedString),
    /// What the result is.
    Caption(SharedString),
    /// A labelled value; `strong` marks the one that is used.
    Entry {
        label: &'static str,
        value: SharedString,
        strong: bool,
    },
}

/// One numbered step of an opened row's calculation.
#[derive(PartialEq)]
pub struct Step {
    pub title: &'static str,
    pub lines: Vec<StepLine>,
}

#[derive(PartialEq)]
pub struct RowDisplay {
    pub cells: Vec<DisplayText>,
    pub steps: Vec<Step>,
    /// What the filter matches, in lower case.
    pub search: String,
    /// Names the record in accessibility labels.
    pub label: SharedString,
}

fn step(index: usize, step: &Step, cx: &App) -> Div {
    let p = *palette(cx);
    v_flex()
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
        .children(step.lines.iter().map(|line| {
            match line {
                StepLine::Formula(text) => div()
                    .text_color(p.muted)
                    .font_features(tabular_digits())
                    .child(text.clone()),
                StepLine::Result(text) => div()
                    .text_size(px(16.))
                    .font_medium()
                    .font_features(tabular_digits())
                    .child(text.clone()),
                StepLine::Caption(text) => div()
                    .mt(px(3.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(text.clone()),
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
                    .child(
                        div()
                            .text_right()
                            .font_features(tabular_digits())
                            .child(value.clone()),
                    ),
            }
        }))
}

/// An opened row: the calculation steps on a raised band, then the record ID
/// with its copy button, and the record's `actions`.
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

/// Placeholders reserve their space at once but stay hidden until `visible`.
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
                    columns
                        .iter()
                        .zip(&layout.columns)
                        .map(|(column, width)| heading(column, *width)),
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
                        .columns
                        .iter()
                        .map(|width| div().w(*width).flex_shrink_0().child(bar(*width))),
                )
        }))
}

/// Placeholders reserve their space at once but stay hidden until `visible`.
pub fn skeleton(columns: &[Column], visible: bool, cx: &App) -> Div {
    v_flex()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .flex_1()
        .min_h(rems(17.))
        .child(
            h_flex()
                .h(px(42.))
                .flex_shrink_0()
                .rounded_t(cx.theme().radius)
                .bg(cx.theme().table_head)
                .border_b_1()
                .border_color(cx.theme().border)
                .children(
                    columns
                        .iter()
                        .map(|column| table::cell(column.label.clone(), column, true, cx)),
                ),
        )
        .children((0..5).map(|_| {
            h_flex()
                .h(px(40.))
                .flex_shrink_0()
                .border_b_1()
                .border_color(cx.theme().border)
                .children(columns.iter().map(|column| {
                    div()
                        .w(rems(column.width / 14.))
                        .flex_shrink_0()
                        .px_3()
                        .py_3()
                        .child(
                            div()
                                .h(px(12.))
                                .w(rems(column.width * 0.6 / 14.))
                                .rounded(px(4.))
                                .bg(cx.theme().skeleton)
                                .opacity(if visible { 1. } else { 0. }),
                        )
                }))
        }))
}

/// A virtual list item: a month header, a record (with its details when
/// opened), or the space after the last row.
#[derive(Clone, Copy, PartialEq)]
pub enum Item {
    Month(usize),
    Record(usize),
    End,
}

/// Identifies an item across rebuilds, for scroll anchoring.
#[derive(Clone, PartialEq)]
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
    /// Shared by every month header.
    pub month: Option<Size<Pixels>>,
    pub expanded: Vec<Option<Size<Pixels>>>,
    pub scroll: VirtualListScrollHandle,
    pub viewport_width: Option<Pixels>,
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
            layout: None,
            layout_key: None,
            dirty: true,
        }
    }
}

/// Reports the width of the stationary table viewport, not the horizontally
/// scrolling table, after layout so that rows can be remeasured.
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
    /// New rows from a reload. The items stay until the next rebuild, so the
    /// first visible one keeps its place if it is still listed.
    pub fn reset(&mut self, rows: Vec<RowDisplay>) {
        self.rows = rows;
        self.invalidate_measurements();
    }

    pub fn invalidate_measurements(&mut self) {
        self.collapsed = None;
        self.month = None;
        self.expanded = vec![None; self.rows.len()];
        self.dirty = true;
    }

    /// Scrolls to the top without anchoring, for a new filter.
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
