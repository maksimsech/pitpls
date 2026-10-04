use gpui_kit::{
    component::{ActiveTheme, VirtualListScrollHandle, h_flex, v_flex},
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
                .bg(cx.theme().muted)
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
pub struct RowDisplay {
    pub cells: Vec<SharedString>,
    pub details: Vec<(SharedString, SharedString)>,
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
