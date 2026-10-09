use super::*;
use crate::components::records::{
    self, END_SPACE, Item, ItemKey, cell, control, details, heading, measure_width,
};
use gpui_kit::{
    Role,
    component::{
        checkbox::Checkbox,
        menu::{DropdownMenu, PopupMenuItem},
        v_virtual_list,
    },
};

impl<K: RecordKind> RecordsPage<K> {
    fn toggle_expanded(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.expanded.remove(id) {
            self.expanded.insert(id.to_owned());
        }
        self.table_state.dirty = true;
        cx.notify();
    }

    fn toggle_folded(&mut self, key: (i32, u32), cx: &mut Context<Self>) {
        if !self.folded.remove(&key) {
            self.folded.insert(key);
        }
        self.table_state.dirty = true;
        cx.notify();
    }

    /// Selects or clears the records at `indices`; other selections stay.
    fn select(&mut self, indices: &[usize], checked: bool) {
        for &index in indices {
            let id = K::id(&self.records[index]);
            if checked {
                self.selected.insert(id.to_owned());
            } else {
                self.selected.remove(id);
            }
        }
    }

    fn all_selected<'a>(&self, mut indices: impl Iterator<Item = &'a usize>) -> bool {
        let mut any = false;
        let all = indices.all(|&index| {
            any = true;
            self.selected.contains(K::id(&self.records[index]))
        });
        any && all
    }

    fn table_header(&self, layout: &TableLayout, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let visible = self
            .groups
            .iter()
            .flat_map(|group| group.records.iter().copied())
            .collect::<Vec<_>>();
        let checked = self.all_selected(visible.iter());
        records::row(layout)
            .h(px(32.))
            .border_t_1()
            .border_b_1()
            .border_color(p.line)
            .text_size(px(12.))
            .text_color(p.faint)
            .child(
                control(layout.frame[0]).child(
                    Checkbox::new("select-all")
                        .accessibility_label("Select all records")
                        .checked(checked)
                        .disabled(self.disabled() || visible.is_empty())
                        .on_click(cx.listener(move |this, checked, _, cx| {
                            this.select(&visible, *checked);
                            cx.notify();
                        })),
                ),
            )
            .child(div().w(layout.frame[1]).flex_shrink_0())
            .children(
                self.columns
                    .iter()
                    .zip(&layout.columns)
                    .map(|(column, width)| heading(column, *width)),
            )
            .child(
                div()
                    .w(layout.frame[2])
                    .flex_shrink_0()
                    .flex()
                    .justify_end()
                    .when(!self.expanded.is_empty(), |cell| {
                        cell.child(
                            Button::new("collapse-all")
                                .ghost()
                                .size(px(24.))
                                .p_0()
                                .rounded(px(6.))
                                .text_color(p.muted)
                                .child(Icon::new(IconName::ChevronsDownUp).size(px(14.)))
                                .accessibility_label("Collapse all")
                                .tooltip("Collapse all")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.expanded.clear();
                                    this.table_state.dirty = true;
                                    cx.notify();
                                })),
                        )
                    }),
            )
    }

    fn chevron(open: bool, cx: &App) -> Icon {
        let p = *palette(cx);
        Icon::new(if open {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        })
        .size(px(14.))
        .text_color(if open { p.text } else { p.faint })
    }

    /// A month header: select the month, fold it, and its subtotal.
    fn month_row(
        &self,
        group: usize,
        layout: &TableLayout,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = *palette(cx);
        let group = &self.groups[group];
        let key = group.key;
        let open = !self.folded.contains(&key);
        let indices = group.records.clone();
        let count = group.records.len();
        records::row(layout)
            .id(SharedString::from(format!("month-{}-{}", key.0, key.1)))
            .h(px(36.))
            .border_b_1()
            .border_color(p.line)
            .cursor_pointer()
            .hover(|row| row.bg(p.hover))
            .role(Role::Button)
            .aria_label(format!(
                "{} {}",
                if open { "Fold" } else { "Unfold" },
                group.title
            ))
            .aria_expanded(open)
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_folded(key, cx)))
            .child(
                control(layout.frame[0]).child(
                    Checkbox::new(SharedString::from(format!(
                        "select-month-{}-{}",
                        key.0, key.1
                    )))
                    .accessibility_label(format!("Select all in {}", group.title))
                    .checked(self.all_selected(group.records.iter()))
                    .disabled(self.disabled())
                    .on_click(cx.listener(move |this, checked, _, cx| {
                        this.select(&indices, *checked);
                        cx.notify();
                    })),
                ),
            )
            .child(Self::chevron(open, cx))
            .child(
                div()
                    .ml(px(8.))
                    .flex_shrink_0()
                    .font_semibold()
                    .child(group.title.clone()),
            )
            .child(div().flex_1().min_w(px(8.)))
            .child(
                h_flex()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .font_features(tabular_digits())
                    .child(if count == 1 {
                        "1 record".to_owned()
                    } else {
                        format!("{count} records")
                    })
                    .children(
                        group
                            .summary
                            .iter()
                            .enumerate()
                            .map(|(index, (label, value))| {
                                h_flex()
                                    .flex_shrink_0()
                                    .child(" · ")
                                    .when_some(*label, |part, label| {
                                        part.child(format!("{label} "))
                                    })
                                    .child(value::reveal_full(
                                        div()
                                            .id(SharedString::from(format!(
                                                "subtotal-{}-{}-{index}",
                                                key.0, key.1
                                            )))
                                            .child(value::text(value, false, true, cx)),
                                        value,
                                    ))
                            }),
                    ),
            )
    }

    /// A record's row: click anywhere to open it. The checkbox and "⋯" are
    /// their own targets.
    fn record_line(
        &self,
        index: usize,
        layout: &TableLayout,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = *palette(cx);
        let display = &self.table_state.rows[index];
        let id = K::id(&self.records[index]);
        let selected = self.selected.contains(id);
        let open = self.expanded.contains(id);
        let disabled = self.disabled();
        let group = SharedString::from(format!("row-{id}"));
        let (select_id, toggle_id) = (id.to_owned(), id.to_owned());
        records::row(layout)
            .id(SharedString::from(format!("line-{id}")))
            .group(group.clone())
            .h(px(40.))
            .text_size(px(13.))
            .border_b_1()
            .border_color(p.line)
            .cursor_pointer()
            .when(selected, |row| row.bg(p.selected))
            .when(!selected, |row| row.hover(|row| row.bg(p.hover)))
            .role(Role::Button)
            .aria_label(format!(
                "{} calculation for {}",
                if open { "Hide" } else { "Show" },
                display.label
            ))
            .aria_expanded(open)
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_expanded(&toggle_id, cx)))
            .child(
                control(layout.frame[0]).child(
                    Checkbox::new(SharedString::from(format!("select-{id}")))
                        .accessibility_label(format!("Select {}", display.label))
                        .checked(selected)
                        .disabled(disabled)
                        .on_click(cx.listener(move |this, checked, _, cx| {
                            if *checked {
                                this.selected.insert(select_id.clone());
                            } else {
                                this.selected.remove(&select_id);
                            }
                            cx.notify();
                        })),
                ),
            )
            .child(
                div()
                    .w(layout.frame[1])
                    .flex_shrink_0()
                    .child(Self::chevron(open, cx)),
            )
            .children(
                display
                    .cells
                    .iter()
                    .zip(&self.columns)
                    .zip(&layout.columns)
                    .enumerate()
                    .map(|(column, ((value, kind), width))| {
                        cell(
                            SharedString::from(format!("cell-{id}-{column}")),
                            value,
                            kind,
                            *width,
                            cx,
                        )
                    }),
            )
            .child(control(layout.frame[2]).justify_end().child(self.more_menu(
                id,
                &display.label,
                open,
                group,
                disabled,
                cx,
            )))
    }

    /// "⋯" with Edit and Delete. It shows on hover and on an opened row.
    fn more_menu(
        &self,
        id: &str,
        label: &SharedString,
        open: bool,
        group: SharedString,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = *palette(cx);
        let view = cx.entity().downgrade();
        let id = id.to_owned();
        Button::new(SharedString::from(format!("more-{id}")))
            .ghost()
            .size(px(24.))
            .p_0()
            .rounded(px(6.))
            .text_color(p.muted)
            .child(Icon::new(IconName::Ellipsis).size(px(14.)))
            .accessibility_label(format!("More actions for {label}"))
            .disabled(disabled)
            .when(!open, |button| {
                button
                    .opacity(0.)
                    .group_hover(group, |style| style.opacity(1.))
            })
            // A dialog gives focus back to what had it when it opened. The
            // menu is gone by the time the dialog closes, so the page takes
            // focus first.
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                let (edit, delete) = (view.clone(), view.clone());
                let (edit_id, delete_id) = (id.clone(), id.clone());
                menu.item(PopupMenuItem::new("Edit").on_click(move |_, window, cx| {
                    let _ = edit.update(cx, |this, cx| {
                        window.focus(&this.focus, cx);
                        this.edit(&edit_id, window, cx);
                    });
                }))
                .item(PopupMenuItem::new("Delete").on_click(move |_, window, cx| {
                    let _ = delete.update(cx, |this, cx| {
                        window.focus(&this.focus, cx);
                        this.delete_one(&delete_id, window, cx);
                    });
                }))
            })
    }

    fn record_row(
        &self,
        index: usize,
        open: bool,
        layout: &TableLayout,
        cx: &mut Context<Self>,
    ) -> Div {
        let row = v_flex().child(self.record_line(index, layout, cx));
        if !open {
            return row;
        }
        let id = K::id(&self.records[index]).to_owned();
        let disabled = self.disabled();
        let p = *palette(cx);
        let action = |name: &'static str, label: &'static str, color: Option<Hsla>| {
            Button::new(SharedString::from(format!("{name}-{id}")))
                .ghost()
                .h(px(26.))
                .px(px(10.))
                .rounded(px(8.))
                .text_color(p.muted)
                .accessibility_label(format!("{label} {}", self.table_state.rows[index].label))
                .disabled(disabled)
                // A colour on the label, since the kit's hover colour
                // replaces the button's own.
                .child(
                    div()
                        .text_size(px(12.))
                        .font_medium()
                        .when_some(color.filter(|_| !disabled), |label, color| {
                            label.text_color(color)
                        })
                        .child(label),
                )
        };
        let (edit_id, delete_id) = (id.clone(), id.clone());
        let edit = action("edit", "Edit", None)
            .on_click(cx.listener(move |this, _, window, cx| this.edit(&edit_id, window, cx)));
        let delete = action("delete", "Delete", Some(p.danger)).on_click(
            cx.listener(move |this, _, window, cx| this.delete_one(&delete_id, window, cx)),
        );
        row.child(details(
            &id,
            &self.table_state.rows[index].steps,
            layout,
            [edit.into_any_element(), delete.into_any_element()],
            cx,
        ))
    }

    fn render_item(
        &self,
        index: usize,
        layout: &TableLayout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.table_state.items[index] {
            Item::Month(group) => self.month_row(group, layout, cx).into_any_element(),
            Item::Record(record) => {
                let id = K::id(&self.records[record]);
                div()
                    .id(SharedString::from(id.to_owned()))
                    .child(self.record_row(record, self.expanded.contains(id), layout, cx))
                    .into_any_element()
            }
            Item::End => div().id("records-end").h(END_SPACE).into_any_element(),
        }
    }

    /// Rebuilds the list items and their heights: month headers, then the
    /// rows of unfolded months. Collapsed rows and headers are measured once;
    /// opened rows once each until the width or fonts change.
    fn rebuild(&mut self, layout: &TableLayout, window: &mut Window, cx: &mut Context<Self>) {
        let available = size(
            AvailableSpace::Definite(layout.width),
            AvailableSpace::MinContent,
        );
        let mut items = Vec::new();
        let mut keys = Vec::new();
        let mut sizes = Vec::new();
        for group in 0..self.groups.len() {
            let key = self.groups[group].key;
            let month = match self.table_state.month {
                Some(size) => size,
                None => {
                    let size = self
                        .month_row(group, layout, cx)
                        .into_any_element()
                        .layout_as_root(available, window, cx);
                    self.table_state.month = Some(size);
                    size
                }
            };
            items.push(Item::Month(group));
            keys.push(ItemKey::Month(key.0, key.1));
            sizes.push(month);
            if self.folded.contains(&key) {
                continue;
            }
            for position in 0..self.groups[group].records.len() {
                let index = self.groups[group].records[position];
                let id = K::id(&self.records[index]);
                let open = self.expanded.contains(id);
                let key = ItemKey::Record(SharedString::from(id.to_owned()));
                let cached = if open {
                    self.table_state.expanded[index]
                } else {
                    self.table_state.collapsed
                };
                let measured = cached.unwrap_or_else(|| {
                    self.record_row(index, open, layout, cx)
                        .into_any_element()
                        .layout_as_root(available, window, cx)
                });
                if open {
                    self.table_state.expanded[index] = Some(measured);
                } else {
                    self.table_state.collapsed = Some(measured);
                }
                items.push(Item::Record(index));
                keys.push(key);
                sizes.push(measured);
            }
        }
        items.push(Item::End);
        keys.push(ItemKey::End);
        sizes.push(size(layout.width, END_SPACE));
        self.table_state.set_items(items, keys, sizes);
    }

    pub(super) fn table(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = *palette(cx);
        let viewport = self.table_state.viewport_width.unwrap_or(px(900.));
        let layout = TableLayout::new(&self.columns, viewport);
        let key = (
            layout.clone(),
            window.rem_size(),
            cx.theme().font_family.clone(),
            cx.theme().mono_font_family.clone(),
        );
        if self.table_state.layout_key.as_ref() != Some(&key) {
            self.table_state.layout_key = Some(key);
            self.table_state.invalidate_measurements();
        }
        self.table_state.layout = Some(layout.clone());
        if self.table_state.dirty {
            self.rebuild(&layout, window, cx);
        }
        let body = if self.groups.is_empty() {
            div()
                .px(px(20.))
                .py(px(24.))
                .text_color(p.muted)
                .child(format!("No records match “{}”.", self.query))
                .into_any_element()
        } else {
            let scroll = self.table_state.scroll.clone();
            let list = v_virtual_list(
                cx.entity(),
                "record-rows",
                self.table_state.sizes.clone(),
                move |this, range, _, cx| {
                    let Some(layout) = this.table_state.layout.clone() else {
                        return vec![];
                    };
                    range
                        .map(|index| this.render_item(index, &layout, cx))
                        .collect()
                },
            )
            .track_scroll(&scroll)
            .with_sizing_behavior(ListSizingBehavior::Auto)
            .flex_1()
            .min_h_0();
            div()
                .relative()
                .flex()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(list)
                .vertical_scrollbar(&scroll)
                .into_any_element()
        };
        let table = v_flex()
            .w(layout.width)
            .h_full()
            .flex_shrink_0()
            .child(self.table_header(&layout, cx).flex_shrink_0())
            .child(body);
        div()
            .id("records-horizontal")
            .w_full()
            .min_w_0()
            .flex_1()
            .min_h(rems(17.))
            .relative()
            .overflow_hidden()
            .child(measure_width(
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
}
