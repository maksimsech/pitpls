use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Disableable, Sizable, StyledExt, VirtualListScrollHandle,
        button::{Button, ButtonVariants},
        h_flex,
        tooltip::Tooltip,
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};
use std::rc::Rc;

pub const SELECT_WIDTH: f32 = 42.;
pub const ACTION_WIDTH: f32 = 108.;

pub fn record_skeleton(mut columns: Vec<super::table::Column>, visible: bool, cx: &App) -> Div {
    columns.insert(0, super::table::Column::text("", SELECT_WIDTH));
    columns.push(super::table::Column::text("", ACTION_WIDTH));
    skeleton(&columns, visible, cx)
}

/// Static placeholders avoid introducing another flashing animation. The header
/// and rows reserve space even during the short indicator delay.
pub fn skeleton(columns: &[super::table::Column], visible: bool, cx: &App) -> Div {
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
                        .map(|column| super::table::cell(column.label.clone(), column, true, cx)),
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

#[derive(PartialEq)]
pub struct DetailGroup {
    pub title: &'static str,
    pub fields: Vec<(SharedString, SharedString)>,
}

#[derive(PartialEq)]
pub struct RowDisplay {
    pub cells: Vec<SharedString>,
    pub details: Vec<DetailGroup>,
}

/// Keep expanded records in the table's visual flow. Groups wrap when needed;
/// the virtual list measures this same layout to reserve the correct row height.
pub fn record_details(record_id: &str, groups: &[DetailGroup], disabled: bool, cx: &App) -> Div {
    h_flex()
        .w_full()
        .flex_wrap()
        .items_start()
        .gap_6()
        .pl(rems(SELECT_WIDTH / 14. + 0.75))
        .pr_5()
        .py_4()
        .border_t_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().table_head)
        .children(groups.iter().enumerate().map(|(index, group)| {
            v_flex()
                .flex_1()
                .min_w(rems(18.))
                .gap_2()
                .child(
                    div()
                        .mb_1()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child(group.title),
                )
                .children(group.fields.iter().map(|(label, value)| {
                    let full_value = value.clone();
                    h_flex()
                        .w_full()
                        .min_h(rems(1.5))
                        .gap_3()
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_color(cx.theme().muted_foreground)
                                .child(label.clone()),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("detail-{record_id}-{label}")))
                                .flex_1()
                                .min_w_0()
                                .text_right()
                                .font_family(cx.theme().mono_font_family.clone())
                                .truncate()
                                .tooltip(move |window, cx| {
                                    Tooltip::new(full_value.clone()).build(window, cx)
                                })
                                .child(value.clone()),
                        )
                }))
                .when(index == 0, |group| {
                    let copy_id = record_id.to_owned();
                    group.child(
                        h_flex()
                            .w_full()
                            .min_h(rems(1.5))
                            .gap_3()
                            .text_color(cx.theme().muted_foreground)
                            .child(div().flex_shrink_0().child("Record ID"))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_right()
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .truncate()
                                    .child(SharedString::from(record_id.to_owned())),
                            )
                            .child(
                                Button::new(SharedString::from(format!("copy-id-{record_id}")))
                                    .icon(IconName::Copy)
                                    .ghost()
                                    .xsmall()
                                    .accessibility_label("Copy full record ID")
                                    .tooltip(format!("Copy record ID: {record_id}"))
                                    .disabled(disabled)
                                    .on_click(move |_, _, cx| {
                                        cx.stop_propagation();
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy_id.clone(),
                                        ));
                                    }),
                            ),
                    )
                })
        }))
}

pub struct RecordTableState {
    pub rows: Vec<RowDisplay>,
    pub sizes: Rc<Vec<Size<Pixels>>>,
    pub measured: Vec<[Option<Size<Pixels>>; 2]>,
    pub scroll: VirtualListScrollHandle,
    pub width: Pixels,
    pub viewport_width: Option<Pixels>,
    pub rem_size: f32,
    pub layout_key: Option<(Pixels, Pixels, SharedString, SharedString)>,
    pub dirty: bool,
}

impl Default for RecordTableState {
    fn default() -> Self {
        Self {
            rows: vec![],
            sizes: Rc::new(vec![]),
            measured: vec![],
            scroll: VirtualListScrollHandle::new(),
            width: px(0.),
            viewport_width: None,
            rem_size: 16.,
            layout_key: None,
            dirty: true,
        }
    }
}

/// Observe the stationary viewport, independently of the horizontally scrolling
/// table. Defer the update until after layout so virtual rows can be remeasured.
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
    pub fn reset(&mut self, rows: Vec<RowDisplay>) {
        self.rows = rows;
        // A reload can reorder or remove IDs. Reset position and measured sizes
        // together with the page's selection/expansion instead of reusing indices.
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.sizes = Rc::new(vec![]);
        self.invalidate_measurements();
    }

    pub fn invalidate_measurements(&mut self) {
        self.measured = vec![[None, None]; self.rows.len()];
        self.dirty = true;
    }

    pub fn set_sizes(&mut self, sizes: Vec<Size<Pixels>>) {
        // Preserve the first visible item's offset when a preceding item changes
        // height (expansion or typography/width changes). Kit clamps at the end.
        let mut top = px(0.);
        let mut offset = self.scroll.offset();
        let first = self.sizes.iter().position(|item| {
            top += item.height;
            top > -offset.y
        });
        if let Some(first) = first {
            let old_top = top - self.sizes[first].height;
            let new_top = sizes.iter().take(first).map(|s| s.height).sum::<Pixels>();
            offset.y -= new_top - old_top;
            self.scroll.set_offset(offset);
        }
        self.sizes = Rc::new(sizes);
        self.dirty = false;
    }
}
