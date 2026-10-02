use super::*;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::prelude::FluentBuilder;

impl<D: RecordDefinition> RecordsPage<D> {
    fn table_header(&self, cx: &mut Context<Self>) -> Div {
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
                            !self.data.rows.is_empty()
                                && self.selected.len() == self.data.rows.len(),
                        )
                        .disabled(self.status.busy || self.confirmation.is_some())
                        .on_click(cx.listener(|this, checked, _, cx| {
                            this.selected = if *checked {
                                this.data.rows.iter().map(|row| row.id.clone()).collect()
                            } else {
                                HashSet::new()
                            };
                            this.notify(cx);
                        })),
                ),
        );
        row.children(
            self.data
                .columns
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

    fn record_row(&self, record: &RecordRow, cx: &mut Context<Self>) -> Div {
        let selected_id = record.id.clone();
        let expand_id = record.id.clone();
        let delete_id = record.id.clone();
        let edit = record.edit.clone();
        let expanded = self.expanded.contains(&record.id);
        let disabled = self.status.busy || self.confirmation.is_some();
        let label = format!("{} record on {}", D::PAGE.title(), record.cells[0]);
        v_flex()
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
                                        this.notify(cx);
                                    })),
                            ),
                    )
                    .children(
                        record
                            .cells
                            .iter()
                            .zip(&self.data.columns)
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
                                        this.notify(cx);
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
                                        let editor = D::form(Some(edit.clone()), window, cx);
                                        this.open_editor(editor, true, window, cx);
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("delete-{}", record.id)))
                                    .icon(IconName::Trash)
                                    .ghost()
                                    .small()
                                    .accessibility_label(format!("Delete {label}"))
                                    .disabled(disabled)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.confirmation = Some(Confirmation {
                                            message: "Delete this record? This cannot be undone."
                                                .into(),
                                            ids: vec![delete_id.clone()],
                                        });
                                        this.notify(cx);
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
                        .children(record.details.iter().map(|(label, value)| {
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

    pub(super) fn records(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.data.rows.is_empty() {
            return self
                .empty_state(
                    "No records for this period. Add a record or import a file.",
                    cx,
                )
                .into_any_element();
        }
        let width = self
            .data
            .columns
            .iter()
            .map(|column| column.width)
            .sum::<f32>()
            + SELECT_WIDTH
            + ACTION_WIDTH;
        let table = v_flex()
            .min_w(rems(width / 14.))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(self.table_header(cx))
            .children(self.data.rows.iter().map(|row| self.record_row(row, cx)))
            .child(
                div()
                    .p_3()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "{} record(s) · Totals above cover the entire selected period.",
                        self.data.rows.len()
                    )),
            );
        div()
            .id("records-horizontal")
            .w_full()
            .relative()
            .overflow_x_scroll()
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
                .disabled(self.status.busy || self.confirmation.is_some())
                .on_click(cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx))),
        )
    }
}
