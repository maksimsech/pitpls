use super::*;
use crate::{
    components::{
        records::{ACTION_WIDTH, SELECT_WIDTH, record_details},
        table::{cell, value_cell},
    },
    format::date,
};
use gpui_kit::{
    assets::IconName,
    component::{checkbox::Checkbox, v_virtual_list},
};

impl<K: RecordKind> RecordsPage<K> {
    /// Numeric columns share the width left over by the minimum layout.
    fn columns(&self) -> Vec<Column> {
        let mut columns = K::columns();
        let available = self.table_state.width / px(1.) * 14. / self.table_state.rem_size;
        let minimum =
            columns.iter().map(|column| column.width).sum::<f32>() + SELECT_WIDTH + ACTION_WIDTH;
        let flexible = columns.iter().filter(|column| column.numeric).count();
        let extra = (available - minimum).max(0.) / flexible.max(1) as f32;
        for column in columns.iter_mut().filter(|column| column.numeric) {
            column.width += extra;
        }
        columns
    }

    fn table_header(&self, columns: &[Column], cx: &mut Context<Self>) -> Div {
        h_flex()
            .h(px(42.))
            .rounded_t(cx.theme().radius)
            .bg(cx.theme().table_head)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .w(rems(SELECT_WIDTH / 14.))
                    .flex_shrink_0()
                    .px_3()
                    .child(
                        Checkbox::new("select-all")
                            .accessibility_label("Select all records")
                            .checked(
                                !self.records.is_empty()
                                    && self.selected.len() == self.records.len(),
                            )
                            .disabled(self.disabled())
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.selected = if *checked {
                                    this.records
                                        .iter()
                                        .map(|record| K::id(record).to_owned())
                                        .collect()
                                } else {
                                    HashSet::new()
                                };
                                cx.notify();
                            })),
                    ),
            )
            .children(
                columns
                    .iter()
                    .map(|column| cell(column.label.clone(), column, true, cx)),
            )
            .child(
                div()
                    .w(rems(ACTION_WIDTH / 14.))
                    .flex_shrink_0()
                    .px_3()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().table_head_foreground)
                    .child("Actions"),
            )
    }

    fn record_row(
        &self,
        record: &K::Record,
        index: usize,
        columns: &[Column],
        cx: &mut Context<Self>,
    ) -> Div {
        let display = &self.table_state.rows[index];
        let id = K::id(record);
        let selected_id = id.to_owned();
        let expand_id = id.to_owned();
        let delete_id = id.to_owned();
        let edit_id = id.to_owned();
        let selected = self.selected.contains(id);
        let expanded = self.expanded.contains(id);
        let disabled = self.disabled();
        let label = format!("{} record on {}", K::NAME, date(K::date(record)).text);
        v_flex()
            .w(self.table_state.width)
            .text_sm()
            .font_family(cx.theme().font_family.clone())
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .min_h(px(40.))
                    .when(selected, |row| row.bg(cx.theme().muted))
                    .child(
                        div()
                            .w(rems(SELECT_WIDTH / 14.))
                            .flex_shrink_0()
                            .px_3()
                            .child(
                                Checkbox::new(SharedString::from(format!("select-{id}")))
                                    .accessibility_label(format!("Select {label}"))
                                    .checked(selected)
                                    .disabled(disabled)
                                    .on_click(cx.listener(move |this, checked, _, cx| {
                                        if *checked {
                                            this.selected.insert(selected_id.clone());
                                        } else {
                                            this.selected.remove(&selected_id);
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .children(display.cells.iter().zip(columns).enumerate().map(
                        |(index, (value, column))| value_cell(("cell", index), value, column, cx),
                    ))
                    .child(
                        h_flex()
                            .w(rems(ACTION_WIDTH / 14.))
                            .flex_shrink_0()
                            .gap_1()
                            .px_2()
                            .child(
                                Button::new(SharedString::from(format!("expand-{id}")))
                                    .icon(if expanded {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    })
                                    .ghost()
                                    .small()
                                    .accessibility_label(format!(
                                        "{} details for {label}",
                                        if expanded { "Hide" } else { "Show" }
                                    ))
                                    .disabled(disabled)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if !this.expanded.remove(&expand_id) {
                                            this.expanded.insert(expand_id.clone());
                                        }
                                        this.table_state.dirty = true;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("edit-{id}")))
                                    .icon(IconName::SquarePen)
                                    .ghost()
                                    .small()
                                    .accessibility_label(format!("Edit {label}"))
                                    .disabled(disabled)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if let Some(record) = this
                                            .records
                                            .iter()
                                            .find(|record| K::id(record) == edit_id)
                                        {
                                            let form = K::Form::new(Some(record), window, cx);
                                            this.open_editor(form, window, cx);
                                        }
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("delete-{id}")))
                                    .icon(IconName::Trash)
                                    .danger()
                                    .small()
                                    .accessibility_label(format!("Delete {label}"))
                                    .disabled(disabled)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_confirmation(
                                            Confirmation {
                                                message:
                                                    "Delete this record? This cannot be undone."
                                                        .into(),
                                                ids: vec![delete_id.clone()],
                                            },
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    ),
            )
            .when(expanded, |row| {
                row.child(record_details(id, &display.details, disabled, cx))
            })
    }

    pub(super) fn records(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.records.is_empty() {
            return self.empty_state(cx).into_any_element();
        }
        let width = K::columns().iter().map(|column| column.width).sum::<f32>()
            + SELECT_WIDTH
            + ACTION_WIDTH;
        // The page can be narrower than the window, so rows are sized to the
        // table viewport and scroll horizontally below the minimum width.
        let minimum = window.rem_size() * (width / 14.);
        let width = self
            .table_state
            .viewport_width
            .unwrap_or(minimum)
            .max(minimum);
        self.table_state.width = width;
        self.table_state.rem_size = window.rem_size() / px(1.);
        let columns = self.columns();
        let key = (
            width,
            window.rem_size(),
            cx.theme().font_family.clone(),
            cx.theme().mono_font_family.clone(),
        );
        if self.table_state.layout_key.as_ref() != Some(&key) {
            self.table_state.layout_key = Some(key);
            self.table_state.invalidate_measurements();
        }
        if self.table_state.dirty {
            let mut sizes = Vec::with_capacity(self.records.len());
            for (index, record) in self.records.iter().enumerate() {
                let expanded = usize::from(self.expanded.contains(K::id(record)));
                let measured = if let Some(measured) = self.table_state.measured[index][expanded] {
                    measured
                } else {
                    let measured = self
                        .record_row(record, index, &columns, cx)
                        .into_any_element()
                        .layout_as_root(
                            size(AvailableSpace::Definite(width), AvailableSpace::MinContent),
                            window,
                            cx,
                        );
                    self.table_state.measured[index][expanded] = Some(measured);
                    measured
                };
                sizes.push(measured);
            }
            self.table_state.set_sizes(sizes);
        }
        let scroll = self.table_state.scroll.clone();
        let list = v_virtual_list(
            cx.entity(),
            "record-rows",
            self.table_state.sizes.clone(),
            move |this, range, _, cx| {
                let columns = this.columns();
                range
                    .map(|index| {
                        let record = &this.records[index];
                        div()
                            .id(SharedString::from(K::id(record).to_owned()))
                            .child(this.record_row(record, index, &columns, cx))
                    })
                    .collect()
            },
        )
        .track_scroll(&scroll)
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .flex_1()
        .min_h_0();
        let table = v_flex()
            .w(width)
            .h_full()
            .flex_shrink_0()
            .child(self.table_header(&columns, cx).flex_shrink_0())
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(list)
                    .vertical_scrollbar(&scroll),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .p_3()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "{} record(s) · {} selected",
                        self.records.len(),
                        self.selected.len()
                    )),
            );
        div()
            .id("records-horizontal")
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .w_full()
            .min_w_0()
            .flex_1()
            .min_h(rems(17.))
            .relative()
            .overflow_hidden()
            .child(components::records::measure_width(
                self.table_state.viewport_width,
                |page: &mut Self| &mut page.table_state,
                cx,
            ))
            .child(
                div()
                    .id("records-pan")
                    .size_full()
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.table_scroll)
                    .child(table),
            )
            .horizontal_scrollbar(&self.table_scroll)
            .into_any_element()
    }

    fn empty_state(&self, cx: &mut Context<Self>) -> Div {
        components::empty(
            "No records for this period. Add a record or import a file.",
            cx,
        )
        .child(
            Button::new("empty-import")
                .label("Open imports")
                .outline()
                .disabled(self.disabled())
                .on_click(cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx))),
        )
    }
}
