use super::*;
use crate::{
    components::table::{Column, cell},
    format::{amount, exact_pln, money},
};
use gpui_kit::{assets::IconName, component::checkbox::Checkbox};
use pitpls_core::crypto::Action;

use crate::components::records::RowDisplay;
use gpui_kit::component::{scroll::ScrollableElement, v_virtual_list};
use gpui_kit::prelude::FluentBuilder;

const SELECT_WIDTH: f32 = 42.;
const ACTION_WIDTH: f32 = 132.;

impl CryptoPage {
    pub(super) fn display_record(record: &CalculatedCrypto) -> RowDisplay {
        let cells: Vec<SharedString> = vec![
            record.date.to_string().into(),
            record.provider.clone().into(),
            match record.action {
                Action::FiatBuy => "Buy",
                Action::FiatSell => "Sell",
            }
            .into(),
            amount(record.value),
            amount(record.fee),
            money(record.calculated_value),
            money(record.calculated_fee),
        ];
        let details: Vec<(SharedString, SharedString)> = vec![
            ("Record ID".into(), record.id.clone().into()),
            ("NBP date".into(), record.nbp_date.to_string().into()),
            ("Original value".into(), amount(record.value)),
            ("Original fee".into(), amount(record.fee)),
            (
                "Calculated value".into(),
                exact_pln(record.calculated_value),
            ),
            ("Calculated fee".into(), exact_pln(record.calculated_fee)),
        ];
        RowDisplay { cells, details }
    }

    fn columns() -> Vec<Column> {
        vec![
            Column::text("Date", 120.),
            Column::text("Provider", 140.),
            Column::text("Action", 85.),
            Column::number("Value", 160.),
            Column::number("Fee", 145.),
            Column::number("Calculated value", 155.),
            Column::number("Calculated fee", 145.),
        ]
    }

    fn table_header(&self, columns: &[Column], cx: &mut Context<Self>) -> Div {
        let mut row = h_flex()
            .h(px(42.))
            .bg(cx.theme().muted)
            .border_b_1()
            .border_color(cx.theme().border);
        row = row.child(
            div()
                .w(rems(SELECT_WIDTH / 14.))
                .flex_shrink_0()
                .px_3()
                .child(
                    Checkbox::new("select-all")
                        .accessibility_label("Select all records")
                        .checked(
                            !self.records.is_empty() && self.selected.len() == self.records.len(),
                        )
                        .disabled(
                            self.status.busy
                                || self.editor.is_some()
                                || self.confirmation.is_some(),
                        )
                        .on_click(cx.listener(|this, checked, _, cx| {
                            this.selected = if *checked {
                                this.records.iter().map(|row| row.id.clone()).collect()
                            } else {
                                HashSet::new()
                            };
                            cx.notify();
                        })),
                ),
        );
        row.children(
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
                .child("Actions"),
        )
    }

    fn record_row(
        &self,
        record: &CalculatedCrypto,
        index: usize,
        columns: &[Column],
        cx: &mut Context<Self>,
    ) -> Div {
        let display = &self.table_state.rows[index];
        let selected_id = record.id.clone();
        let expand_id = record.id.clone();
        let delete_id = record.id.clone();
        let edit_id = record.id.clone();
        let expanded = self.expanded.contains(&record.id);
        let disabled = self.status.busy || self.editor.is_some() || self.confirmation.is_some();
        let label = format!("Crypto record on {}", record.date);
        v_flex()
            .w(self.table_state.width)
            .text_sm()
            .font_family(cx.theme().font_family.clone())
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .min_h(px(48.))
                    .when(self.selected.contains(&record.id), |row| {
                        row.bg(cx.theme().muted)
                    })
                    .child(
                        div()
                            .w(rems(SELECT_WIDTH / 14.))
                            .flex_shrink_0()
                            .px_3()
                            .child(
                                Checkbox::new(SharedString::from(format!("select-{}", record.id)))
                                    .accessibility_label(format!("Select {label}"))
                                    .checked(self.selected.contains(&record.id))
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
                    .children(
                        display
                            .cells
                            .iter()
                            .zip(columns)
                            .map(|(value, column)| cell(value.clone(), column, false, cx)),
                    )
                    .child(
                        h_flex()
                            .w(rems(ACTION_WIDTH / 14.))
                            .flex_shrink_0()
                            .gap_1()
                            .px_2()
                            .child(
                                Button::new(SharedString::from(format!("expand-{}", record.id)))
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
                                Button::new(SharedString::from(format!("edit-{}", record.id)))
                                    .icon(IconName::SquarePen)
                                    .ghost()
                                    .small()
                                    .accessibility_label(format!("Edit {label}"))
                                    .disabled(disabled)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if let Some(record) =
                                            this.records.iter().find(|record| record.id == edit_id)
                                        {
                                            let editor = CryptoForm::new(Some(record), window, cx);
                                            this.open_editor(editor, window, cx);
                                        }
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("delete-{}", record.id)))
                                    .icon(IconName::Trash)
                                    .ghost()
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
                row.child(
                    h_flex()
                        .flex_wrap()
                        .items_start()
                        .gap_4()
                        .p_5()
                        .bg(cx.theme().muted)
                        .children(display.details.iter().map(|(label, value)| {
                            v_flex()
                                .w(px(280.))
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(label.clone()),
                                )
                                .child(
                                    div()
                                        .font_family(cx.theme().mono_font_family.clone())
                                        .child(value.clone()),
                                )
                        })),
                )
            })
    }

    pub fn records(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.records.is_empty() {
            return self
                .empty_state(
                    "No records for this period. Add a record or import a file.",
                    cx,
                )
                .into_any_element();
        }
        let columns = Self::columns();
        let width =
            columns.iter().map(|column| column.width).sum::<f32>() + SELECT_WIDTH + ACTION_WIDTH;
        // Match the page padding and table borders. The width is explicit in both
        // measurement and presentation, including horizontally overflowing tables.
        let width = (window.viewport_size().width - window.rem_size() * 2.5 - px(2.))
            .max(window.rem_size() * (width / 14.));
        let key = (
            width,
            window.rem_size(),
            cx.theme().font_family.clone(),
            cx.theme().mono_font_family.clone(),
        );
        if self.table_state.layout_key.as_ref() != Some(&key) {
            self.table_state.layout_key = Some(key);
            self.table_state.width = width;
            self.table_state.invalidate_measurements();
        }
        if self.table_state.dirty {
            let mut sizes = Vec::with_capacity(self.records.len());
            for (index, record) in self.records.iter().enumerate() {
                let expanded = usize::from(self.expanded.contains(&record.id));
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
                let columns = Self::columns();
                range
                    .map(|index| {
                        let record = &this.records[index];
                        div()
                            .id(SharedString::from(record.id.clone()))
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
            .w(width + px(2.))
            .h_full()
            .flex_shrink_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(self.table_header(&columns, cx).flex_shrink_0())
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    // Explicit flex/overflow containment keeps the list's
                    // viewport independent of the total height of its records.
                    .flex()
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
            .w_full()
            .flex_1()
            .min_h(rems(17.))
            .relative()
            .overflow_x_scroll()
            .lock_scroll_axis()
            .track_scroll(&self.table_scroll)
            .child(table)
            .horizontal_scrollbar(&self.table_scroll)
            .into_any_element()
    }

    fn empty_state(&self, message: &'static str, cx: &mut Context<Self>) -> Div {
        components::empty(message, cx).child(
            Button::new("empty-import")
                .label("Open imports")
                .outline()
                .disabled(self.status.busy || self.editor.is_some() || self.confirmation.is_some())
                .on_click(cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx))),
        )
    }
}
