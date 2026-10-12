use crate::{
    components::{
        header, measure_width,
        records::{
            self, END_SPACE, Item, ItemKey, RecordColumn, RecordTableState, RowDisplay,
            TableLayout, cell, control, details, focus_ring, heading, record_skeleton,
        },
        spinner, value,
    },
    format::DisplayText,
    theme::{palette, tabular_digits},
};
use chrono::Datelike;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    Role,
    assets::IconName,
    component::{
        button::*,
        checkbox::Checkbox,
        menu::{DropdownMenu, PopupMenuItem},
        scroll::ScrollableElement,
        v_virtual_list, *,
    },
    *,
};
use pitpls_core::time;
use std::collections::{HashMap, HashSet};

pub struct MonthGroup {
    pub key: (i32, u32),
    pub title: SharedString,
    pub records: Vec<usize>,
    pub count: SharedString,
    pub summary: Vec<(Option<&'static str>, DisplayText)>,
}

/// Keyboard focus entering the item scrolls it into view, so Tab walks on past
/// the rows on screen.
struct ItemFocus {
    handle: FocusHandle,
    _reveal: Subscription,
}

pub struct RecordTable {
    name: &'static str,
    pub columns: Vec<RecordColumn>,
    selectable: bool,
    pub groups: Vec<MonthGroup>,
    folded: HashSet<(i32, u32)>,
    expanded: HashSet<String>,
    pub selected: HashSet<String>,
    pub state: RecordTableState,
    scroll: ScrollHandle,
    item_focus: HashMap<ItemKey, ItemFocus>,
    item_handles: Vec<Option<FocusHandle>>,
}

impl RecordTable {
    /// `name` keeps the element IDs of two tables on one page apart.
    pub fn new(
        name: &'static str,
        columns: Vec<RecordColumn>,
        selectable: bool,
        width: Option<Pixels>,
    ) -> Self {
        let mut state = RecordTableState::default();
        // So the first frame doesn't lay the table out at a guessed width.
        state.viewport_width = width;
        Self {
            name,
            columns,
            selectable,
            groups: Vec::new(),
            folded: HashSet::new(),
            expanded: HashSet::new(),
            selected: HashSet::new(),
            state,
            scroll: ScrollHandle::default(),
            item_focus: HashMap::new(),
            item_handles: Vec::new(),
        }
    }

    pub fn set_rows(&mut self, rows: Vec<RowDisplay>) -> bool {
        if rows == self.state.rows {
            return false;
        }
        let ids = rows
            .iter()
            .map(|row| row.id.as_ref())
            .collect::<HashSet<_>>();
        self.selected.retain(|id| ids.contains(id.as_str()));
        self.expanded.retain(|id| ids.contains(id.as_str()));
        self.state.reset(rows);
        true
    }

    /// `count` and `summary` are left for the caller.
    pub fn month_groups(&self, query: &str, with_year: bool) -> Vec<MonthGroup> {
        let mut groups: Vec<MonthGroup> = Vec::new();
        for (index, row) in self.state.rows.iter().enumerate() {
            if !row.search.contains(query) {
                continue;
            }
            let local = time::local(row.time);
            let key = (local.year(), local.month());
            match groups.last_mut() {
                Some(group) if group.key == key => group.records.push(index),
                _ => groups.push(MonthGroup {
                    key,
                    title: month_title(key, with_year),
                    records: vec![index],
                    count: SharedString::default(),
                    summary: vec![],
                }),
            }
        }
        groups
    }

    pub fn set_groups(&mut self, groups: Vec<MonthGroup>) {
        self.groups = groups;
        self.state.dirty = true;
    }

    pub fn shown(&self) -> usize {
        self.groups.iter().map(|group| group.records.len()).sum()
    }

    pub fn visible(&self) -> impl Iterator<Item = &str> {
        self.groups
            .iter()
            .flat_map(|group| &group.records)
            .map(|&index| self.state.rows[index].id.as_ref())
    }

    pub fn compact(&self) -> bool {
        self.state
            .layout
            .as_ref()
            .is_some_and(|layout| layout.compact)
    }

    pub fn skeleton(&self, visible: bool, cx: &App) -> (Div, bool) {
        let layout = TableLayout::new(
            &self.columns,
            &[],
            self.state.viewport_width.unwrap_or(px(900.)),
            self.selectable,
        );
        (
            record_skeleton(&self.columns, &layout, visible, cx),
            layout.compact,
        )
    }

    fn toggle_expanded(&mut self, id: &str) {
        if !self.expanded.remove(id) {
            self.expanded.insert(id.to_owned());
        }
        self.state.dirty = true;
    }

    fn toggle_folded(&mut self, key: (i32, u32)) {
        if !self.folded.remove(&key) {
            self.folded.insert(key);
        }
        self.state.dirty = true;
    }

    fn select(&mut self, indices: &[usize], checked: bool) {
        for &index in indices {
            let id = &self.state.rows[index].id;
            if checked {
                self.selected.insert(id.to_string());
            } else {
                self.selected.remove(id.as_ref());
            }
        }
    }

    fn all_selected<'a>(&self, mut indices: impl Iterator<Item = &'a usize>) -> bool {
        let mut any = false;
        let all = indices.all(|&index| {
            any = true;
            self.selected.contains(self.state.rows[index].id.as_ref())
        });
        any && all
    }
}

pub trait TableHost: Render {
    type Table: Copy + 'static;

    fn table(&self, which: Self::Table) -> &RecordTable;
    fn table_mut(&mut self, which: Self::Table) -> &mut RecordTable;
    fn disabled(&self) -> bool;
    fn deleting(&self) -> &HashSet<String>;
    fn page_focus(&self) -> &FocusHandle;
    fn editable(&self) -> bool;
    fn edit(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>);
    fn delete_one(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>);
    /// `index` is the link's place in `RowDisplay::links`; a lot table's link
    /// is 0.
    fn follow(
        &mut self,
        _which: Self::Table,
        _id: &str,
        _index: usize,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

pub fn month_title((year, month): (i32, u32), with_year: bool) -> SharedString {
    let name = u8::try_from(month)
        .ok()
        .and_then(|month| chrono::Month::try_from(month).ok())
        .map_or("", |month| month.name());
    if with_year {
        format!("{name} {year}").into()
    } else {
        name.into()
    }
}

fn ensure_focus<V: TableHost>(
    host: &mut V,
    which: V::Table,
    key: &ItemKey,
    window: &mut Window,
    cx: &mut Context<V>,
) -> FocusHandle {
    if let Some(focus) = host.table(which).item_focus.get(key) {
        return focus.handle.clone();
    }
    let handle = cx.focus_handle().tab_stop(true);
    let reveal_key = key.clone();
    let reveal = cx.on_focus_in(&handle, window, move |this: &mut V, window, cx| {
        if window.last_input_was_keyboard() {
            this.table(which).state.reveal(&reveal_key);
            cx.notify();
        }
    });
    host.table_mut(which).item_focus.insert(
        key.clone(),
        ItemFocus {
            handle: handle.clone(),
            _reveal: reveal,
        },
    );
    handle
}

fn table_header<V: TableHost>(
    host: &V,
    which: V::Table,
    layout: &TableLayout,
    cx: &mut Context<V>,
) -> Div {
    let p = *palette(cx);
    let table = host.table(which);
    records::row(layout)
        .h(px(32.))
        .border_t_1()
        .border_b_1()
        .border_color(p.line)
        .text_size(px(12.))
        .text_color(p.faint)
        .when(table.selectable, |row| {
            let visible = table
                .groups
                .iter()
                .flat_map(|group| group.records.iter().copied())
                .collect::<Vec<_>>();
            let checked = table.all_selected(visible.iter());
            row.child(
                control(layout.frame[0]).child(
                    Checkbox::new("select-all")
                        .accessibility_label("Select all records")
                        .checked(checked)
                        .disabled(host.disabled() || visible.is_empty())
                        .on_click(cx.listener(move |this: &mut V, checked, _, cx| {
                            this.table_mut(which).select(&visible, *checked);
                            cx.notify();
                        })),
                ),
            )
        })
        .child(div().w(layout.frame[1]).flex_shrink_0())
        .children(
            layout
                .shown(&table.columns)
                .map(|(column, width)| heading(column, width)),
        )
        .child(
            div()
                .w(layout.frame[2])
                .flex_shrink_0()
                .flex()
                .justify_end()
                .when(!table.expanded.is_empty(), |cell| {
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
                            .on_click(cx.listener(move |this: &mut V, _, _, cx| {
                                let table = this.table_mut(which);
                                table.expanded.clear();
                                table.state.dirty = true;
                                cx.notify();
                            })),
                    )
                }),
        )
}

fn month_row<V: TableHost>(
    host: &V,
    which: V::Table,
    group: usize,
    layout: &TableLayout,
    focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<V>,
) -> Stateful<Div> {
    let p = *palette(cx);
    let table = host.table(which);
    let group = &table.groups[group];
    let key = group.key;
    let open = !table.folded.contains(&key);
    let indices = group.records.clone();
    let counted = !layout.compact;
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
        .when_some(focus, |row, focus| row.track_focus(focus))
        .on_click(cx.listener(move |this: &mut V, _, _, cx| {
            this.table_mut(which).toggle_folded(key);
            cx.notify();
        }))
        .when(table.selectable, |row| {
            row.child(
                control(layout.frame[0]).child(
                    Checkbox::new(SharedString::from(format!(
                        "select-month-{}-{}",
                        key.0, key.1
                    )))
                    .accessibility_label(format!("Select all in {}", group.title))
                    .checked(table.all_selected(group.records.iter()))
                    .disabled(host.disabled())
                    .on_click(cx.listener(
                        move |this: &mut V, checked, _, cx| {
                            this.table_mut(which).select(&indices, *checked);
                            cx.notify();
                        },
                    )),
                ),
            )
        })
        .child(chevron(open, cx))
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
                .when(counted, |summary| summary.child(group.count.clone()))
                .children(
                    group
                        .summary
                        .iter()
                        .enumerate()
                        .map(|(index, (label, value))| {
                            h_flex()
                                .flex_shrink_0()
                                .when(counted || index > 0, |part| part.child(" · "))
                                .when_some(*label, |part, label| part.child(format!("{label} ")))
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
        .when(ring(focus, window), |row| {
            row.relative().child(focus_ring(cx))
        })
}

fn record_line<V: TableHost>(
    host: &V,
    which: V::Table,
    index: usize,
    layout: &TableLayout,
    focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<V>,
) -> Stateful<Div> {
    let p = *palette(cx);
    let table = host.table(which);
    let display = &table.state.rows[index];
    let id = display.id.as_ref();
    let selected = table.selected.contains(id);
    let open = table.expanded.contains(id);
    let disabled = host.disabled();
    let group = SharedString::from(format!("row-{id}"));
    let (select_id, toggle_id) = (id.to_owned(), id.to_owned());
    records::row(layout)
        .id(SharedString::from(format!("line-{id}")))
        .group(group.clone())
        .h(px(40.))
        .text_size(records::ROW_TEXT)
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
        .when_some(focus, |row, focus| row.track_focus(focus))
        .on_click(cx.listener(move |this: &mut V, _, _, cx| {
            this.table_mut(which).toggle_expanded(&toggle_id);
            cx.notify();
        }))
        .when(table.selectable, |row| {
            row.child(
                control(layout.frame[0]).child(
                    Checkbox::new(SharedString::from(format!("select-{id}")))
                        .accessibility_label(format!("Select {}", display.label))
                        .checked(selected)
                        .disabled(disabled)
                        .on_click(cx.listener(move |this: &mut V, checked, _, cx| {
                            let selected = &mut this.table_mut(which).selected;
                            if *checked {
                                selected.insert(select_id.clone());
                            } else {
                                selected.remove(&select_id);
                            }
                            cx.notify();
                        })),
                ),
            )
        })
        .child(
            div()
                .w(layout.frame[1])
                .flex_shrink_0()
                .child(chevron(open, cx)),
        )
        .children(
            layout
                .shown(display.cells.iter().zip(&table.columns).enumerate())
                .map(|((column, (value, kind)), width)| {
                    cell(
                        SharedString::from(format!("cell-{id}-{column}")),
                        value,
                        kind,
                        width,
                        layout.compact,
                        display.flagged.contains(&column),
                        cx,
                    )
                }),
        )
        .child(control(layout.frame[2]).justify_end().map(|slot| {
            if host.deleting().contains(id) {
                slot.child(
                    div()
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(spinner().color(p.muted)),
                )
            } else {
                slot.child(more_menu(
                    host,
                    id,
                    &display.label,
                    open,
                    group,
                    focus.cloned(),
                    disabled,
                    cx,
                ))
            }
        }))
        .when(ring(focus, window), |row| {
            row.relative().child(focus_ring(cx))
        })
}

/// On a closed row it's hidden, so it's no Tab stop; opening the row shows
/// its actions.
#[allow(clippy::too_many_arguments)]
fn more_menu<V: TableHost>(
    host: &V,
    id: &str,
    label: &SharedString,
    open: bool,
    group: SharedString,
    row_focus: Option<FocusHandle>,
    disabled: bool,
    cx: &mut Context<V>,
) -> impl IntoElement {
    let p = *palette(cx);
    let view = cx.entity().downgrade();
    let id = id.to_owned();
    let editable = host.editable();
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
                .tab_stop(false)
        })
        // A dialog gives focus back to what had it when it opened. The
        // menu is gone by the time the dialog closes, so the row, or the
        // page, takes focus first.
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
            let (edit, delete) = (view.clone(), view.clone());
            let (edit_id, delete_id) = (id.clone(), id.clone());
            let (edit_focus, delete_focus) = (row_focus.clone(), row_focus.clone());
            menu.when(editable, |menu| {
                menu.item(PopupMenuItem::new("Edit").on_click(move |_, window, cx| {
                    let _ = edit.update(cx, |this: &mut V, cx| {
                        let focus = edit_focus.clone().unwrap_or(this.page_focus().clone());
                        window.focus(&focus, cx);
                        this.edit(&edit_id, window, cx);
                    });
                }))
            })
            .item(PopupMenuItem::new("Delete").on_click(move |_, window, cx| {
                let _ = delete.update(cx, |this: &mut V, cx| {
                    let focus = delete_focus.clone().unwrap_or(this.page_focus().clone());
                    window.focus(&focus, cx);
                    this.delete_one(&delete_id, window, cx);
                });
            }))
        })
}

fn record_row<V: TableHost>(
    host: &V,
    which: V::Table,
    index: usize,
    layout: &TableLayout,
    focus: Option<&FocusHandle>,
    window: &Window,
    cx: &mut Context<V>,
) -> Div {
    let row = v_flex().child(record_line(host, which, index, layout, focus, window, cx));
    let table = host.table(which);
    let display = &table.state.rows[index];
    if !table.expanded.contains(display.id.as_ref()) {
        return row;
    }
    let id = display.id.to_string();
    let disabled = host.disabled();
    let p = *palette(cx);
    let action = |name: &'static str, label: &'static str, color: Option<Hsla>| {
        Button::new(SharedString::from(format!("{name}-{id}")))
            .ghost()
            .h(px(26.))
            .px(px(10.))
            .rounded(px(8.))
            .text_color(p.muted)
            .accessibility_label(format!("{label} {}", display.label))
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
    let edit = host.editable().then(|| {
        action("edit", "Edit", None)
            .on_click(
                cx.listener(move |this: &mut V, _, window, cx| this.edit(&edit_id, window, cx)),
            )
            .into_any_element()
    });
    let delete = action("delete", "Delete", Some(p.danger)).on_click(
        cx.listener(move |this: &mut V, _, window, cx| this.delete_one(&delete_id, window, cx)),
    );
    let link = |name: &str, index: usize, label: &SharedString| {
        let id = id.clone();
        records::link(
            SharedString::from(format!("{name}-{index}-{id}")),
            label.clone(),
            cx,
        )
        .disabled(disabled)
        .on_click(cx.listener(move |this: &mut V, _, window, cx| {
            this.follow(which, &id, index, window, cx)
        }))
        .into_any_element()
    };
    let links = display
        .links
        .iter()
        .enumerate()
        .map(|(index, label)| link("link", index, label))
        .collect();
    let lots_link = display
        .lots
        .as_ref()
        .and_then(|lots| lots.link.as_ref())
        .map(|label| link("lots-link", 0, label));
    row.child(details(
        display,
        layout,
        links,
        lots_link,
        edit.into_iter().chain([delete.into_any_element()]),
        cx,
    ))
}

fn render_item<V: TableHost>(
    host: &V,
    which: V::Table,
    index: usize,
    layout: &TableLayout,
    window: &Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let table = host.table(which);
    let focus = table.item_handles.get(index).and_then(Option::as_ref);
    match table.state.items[index] {
        Item::Month(group) => {
            month_row(host, which, group, layout, focus, window, cx).into_any_element()
        }
        Item::Record(record) => div()
            .id(table.state.rows[record].id.clone())
            .child(record_row(host, which, record, layout, focus, window, cx))
            .into_any_element(),
        Item::End => div().id("records-end").h(END_SPACE).into_any_element(),
    }
}

/// Collapsed rows and month headers are measured once; opened rows once
/// each until the width or fonts change.
fn rebuild<V: TableHost>(
    host: &mut V,
    which: V::Table,
    layout: &TableLayout,
    window: &mut Window,
    cx: &mut Context<V>,
) {
    let available = size(
        AvailableSpace::Definite(layout.width),
        AvailableSpace::MinContent,
    );
    let mut items = Vec::new();
    let mut keys = Vec::new();
    let mut sizes = Vec::new();
    let mut handles = Vec::new();
    for group in 0..host.table(which).groups.len() {
        let key = host.table(which).groups[group].key;
        let month_key = ItemKey::Month(key.0, key.1);
        handles.push(Some(ensure_focus(host, which, &month_key, window, cx)));
        let month = match host.table(which).state.month {
            Some(size) => size,
            None => {
                let size = month_row(host, which, group, layout, None, window, cx)
                    .into_any_element()
                    .layout_as_root(available, window, cx);
                host.table_mut(which).state.month = Some(size);
                size
            }
        };
        items.push(Item::Month(group));
        keys.push(month_key);
        sizes.push(month);
        if host.table(which).folded.contains(&key) {
            continue;
        }
        for position in 0..host.table(which).groups[group].records.len() {
            let table = host.table(which);
            let index = table.groups[group].records[position];
            let id = table.state.rows[index].id.clone();
            let open = table.expanded.contains(id.as_ref());
            let key = ItemKey::Record(id);
            handles.push(Some(ensure_focus(host, which, &key, window, cx)));
            let table = host.table(which);
            let cached = if open {
                table.state.expanded[index]
            } else {
                table.state.collapsed
            };
            let measured = cached.unwrap_or_else(|| {
                record_row(host, which, index, layout, None, window, cx)
                    .into_any_element()
                    .layout_as_root(available, window, cx)
            });
            let state = &mut host.table_mut(which).state;
            if open {
                state.expanded[index] = Some(measured);
            } else {
                state.collapsed = Some(measured);
            }
            items.push(Item::Record(index));
            keys.push(key);
            sizes.push(measured);
        }
    }
    items.push(Item::End);
    keys.push(ItemKey::End);
    sizes.push(size(layout.width, END_SPACE));
    handles.push(None);
    let table = host.table_mut(which);
    let shown = keys.iter().collect::<HashSet<_>>();
    table.item_focus.retain(|key, _| shown.contains(key));
    table.item_handles = handles;
    table.state.set_items(items, keys, sizes);
}

pub fn table<V: TableHost>(
    host: &mut V,
    which: V::Table,
    empty: impl IntoElement,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let table = host.table_mut(which);
    let viewport = table.state.viewport_width.unwrap_or(px(900.));
    if table.state.fit.is_none() {
        let fit = records::fit_widths(&table.columns, &table.state.rows, window, cx);
        table.state.fit = Some(fit);
    }
    let fit = table.state.fit.as_deref().unwrap_or_default();
    let layout = TableLayout::new(&table.columns, fit, viewport, table.selectable);
    let key = (
        layout.clone(),
        window.rem_size(),
        cx.theme().font_family.clone(),
        cx.theme().mono_font_family.clone(),
    );
    if table.state.layout_key.as_ref() != Some(&key) {
        table.state.layout_key = Some(key);
        table.state.invalidate_measurements();
    }
    table.state.layout = Some(layout.clone());
    if table.state.dirty {
        rebuild(host, which, &layout, window, cx);
    }
    let table = host.table(which);
    let body = if table.groups.is_empty() {
        empty.into_any_element()
    } else {
        let scroll = table.state.scroll.clone();
        let list = v_virtual_list(
            cx.entity(),
            "record-rows",
            table.state.sizes.clone(),
            move |this: &mut V, range, window, cx| {
                let Some(layout) = this.table(which).state.layout.clone() else {
                    return vec![];
                };
                range
                    .map(|index| render_item(this, which, index, &layout, window, cx))
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
    let rows = v_flex()
        .w(layout.width)
        .h_full()
        .flex_shrink_0()
        .child(table_header(host, which, &layout, cx).flex_shrink_0())
        .child(body);
    div()
        .id(host.table(which).name)
        .w_full()
        .min_w_0()
        .flex_1()
        .min_h(rems(17.))
        .relative()
        .overflow_hidden()
        .child(measure_width(
            host.table(which).state.viewport_width,
            move |host: &mut V| &mut host.table_mut(which).state.viewport_width,
            cx,
        ))
        .child(
            div()
                .id("records-pan")
                .size_full()
                .overflow_x_scroll()
                .lock_scroll_axis()
                .track_scroll(&host.table(which).scroll)
                .child(rows),
        )
        .horizontal_scrollbar(&host.table(which).scroll)
        .into_any_element()
}

/// The count includes records the filter hides.
pub fn selection_bar<V: TableHost>(
    host: &V,
    which: V::Table,
    delete_selected: fn(&mut V, &mut Window, &mut Context<V>),
    cx: &mut Context<V>,
) -> Div {
    let p = *palette(cx);
    let disabled = host.disabled();
    let danger = if disabled { p.muted } else { p.danger };
    let selected = &host.table(which).selected;
    let deleting = host.deleting();
    let removing = !deleting.is_empty() && deleting == selected;
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom(px(18.))
        .flex()
        .justify_center()
        .child(
            h_flex()
                .id("selection-bar")
                .h(px(40.))
                .pl(px(14.))
                .pr(px(6.))
                .gap(px(6.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.strong_line)
                .bg(p.popover)
                .shadow_lg()
                .child(
                    div()
                        .mr(px(8.))
                        .font_medium()
                        .font_features(tabular_digits())
                        .whitespace_nowrap()
                        .child(format!("{} selected", selected.len())),
                )
                .child(
                    Button::new("delete-selected")
                        .h(px(28.))
                        .px(px(11.))
                        .rounded(px(8.))
                        .accessibility_label("Remove selected")
                        .disabled(disabled)
                        // On the children: the kit's hover colour replaces
                        // the button's own.
                        .map(|button| {
                            if removing {
                                button.child(spinner().color(danger))
                            } else {
                                button.child(
                                    Icon::new(IconName::Trash).size(px(14.)).text_color(danger),
                                )
                            }
                        })
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_medium()
                                .text_color(danger)
                                .child("Remove selected"),
                        )
                        .on_click(cx.listener(move |this: &mut V, _, window, cx| {
                            delete_selected(this, window, cx)
                        })),
                )
                .child(
                    header::icon_button("clear-selection", IconName::X, "Clear selection")
                        .text_color(p.muted)
                        .disabled(disabled)
                        .on_click(cx.listener(move |this: &mut V, _, _, cx| {
                            this.table_mut(which).selected.clear();
                            cx.notify();
                        })),
                ),
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

fn ring(focus: Option<&FocusHandle>, window: &Window) -> bool {
    window.last_input_was_keyboard() && focus.is_some_and(|focus| focus.is_focused(window))
}
