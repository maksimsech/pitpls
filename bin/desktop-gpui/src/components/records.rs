use gpui_kit::{component::VirtualListScrollHandle, *};
use std::rc::Rc;

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
            layout_key: None,
            dirty: true,
        }
    }
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
