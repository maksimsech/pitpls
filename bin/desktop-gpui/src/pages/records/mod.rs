mod crypto;
mod dividends;
mod interests;
mod table;

pub use self::{crypto::Crypto, dividends::Dividends, interests::Interests};

use super::{PageView, missing_rate};
use crate::{
    components::{
        Status, dialog, form, header, nbp, notice,
        records::{
            Preview, RecordColumn, RecordTableState, RowDisplay, TableLayout, preview_band,
            record_skeleton,
        },
        value,
    },
    format::{DisplayText, pln},
    navigation::{Page, PageContext},
    theme::{palette, tabular_digits},
};
use chrono::{Datelike, NaiveDate};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        button::*,
        collapsible::Collapsible,
        dialog::Dialog,
        empty::{Empty, EmptyContent},
        input::{Input, InputEvent, InputState},
        scroll::ScrollableElement,
        *,
    },
    *,
};
use pitpls_app::use_case::{
    rate,
    year::{self, YearInfo},
};
use pitpls_core::common::Amount;
use pitpls_importers::{IMPORTERS, OutputType};
use rust_decimal::Decimal;
use std::{collections::HashSet, future::Future, sync::Arc};

pub enum Submission<C, U> {
    Create(C),
    Update(U),
}

pub trait RecordForm: 'static {
    type Record;
    type Submission: Send + 'static;
    /// The values the calculation reads, for the preview.
    type Draft: Clone + PartialEq + Send + 'static;

    fn new(record: Option<&Self::Record>, window: &mut Window, cx: &mut App) -> Self;
    fn title(&self) -> &'static str;
    /// The record being edited, or `None` when adding one.
    fn existing_id(&self) -> Option<&str>;
    /// The optional ID, folded away below the other fields.
    fn id_input(&self) -> &Entity<InputState>;
    /// The input focused when the editor opens. Enter opens a focused date
    /// picker, so this is a text input, where Enter saves.
    fn first_input(&self) -> &Entity<InputState>;
    fn submission(&self, cx: &App) -> Result<Self::Submission, String>;
    /// The values the calculation reads, once every one of them is valid.
    fn draft(&self, cx: &App) -> Option<Self::Draft>;
    /// Calls `changed` on the view whenever a field changes.
    fn watch<V: 'static>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
        changed: fn(&mut V, &mut Window, &mut Context<V>),
    ) -> Vec<Subscription>;
    /// Every field but the ID, in two columns.
    fn render(&self, busy: bool, cx: &App) -> Div;
}

type Draft<K> = <<K as RecordKind>::Form as RecordForm>::Draft;

pub trait RecordKind: 'static {
    type Record: Send + 'static;
    type Form: RecordForm<Record = Self::Record>;

    const PAGE: Page;
    /// Names a single record in accessibility labels and year links.
    const NAME: &'static str;
    /// Names the records in the empty state: "No dividends in 2026".
    const PLURAL: &'static str;
    const TOTAL_LABELS: &'static [&'static str];

    fn id(record: &Self::Record) -> &str;
    fn date(record: &Self::Record) -> NaiveDate;
    fn columns() -> Vec<RecordColumn>;
    fn display(record: &Self::Record) -> RowDisplay;
    /// A month header's sums of its records' calculated values, each with an
    /// optional label. They are information only: summed here at full
    /// precision, never by the calculation.
    fn subtotal(records: &[&Self::Record]) -> Vec<(Option<&'static str>, Decimal)>;
    /// Whether an importer output fills this page.
    fn imported(output: &OutputType) -> bool;
    /// How many of this page's records a year has.
    fn count(year: &YearInfo) -> u32;

    /// Loads the totals, in `TOTAL_LABELS` order, and the records.
    fn load(
        app: Arc<pitpls_app::App>,
        year: Option<i32>,
    ) -> impl Future<Output = Result<(Vec<Decimal>, Vec<Self::Record>), String>> + Send;
    fn save(
        app: Arc<pitpls_app::App>,
        submission: <Self::Form as RecordForm>::Submission,
    ) -> impl Future<Output = Result<(), String>> + Send;
    fn delete(
        app: Arc<pitpls_app::App>,
        ids: Vec<String>,
    ) -> impl Future<Output = Result<u64, String>> + Send;
    /// Calculates a draft as the page would once it is saved, without saving.
    fn preview(
        app: Arc<pitpls_app::App>,
        draft: Draft<Self>,
    ) -> impl Future<Output = Result<Self::Record, String>> + Send;
    /// The editor's preview band for a calculated draft.
    fn preview_display(record: &Self::Record) -> Preview;
}

/// `value × rate`, every digit, for an opened row's conversion step.
fn conversion(value: Amount, rate: Decimal) -> String {
    format!("{} × {rate}", crate::format::amount(value).full)
}

/// A rate such as 0.19 as `19%`.
fn percent(rate: Decimal) -> String {
    format!("{}%", (rate * Decimal::ONE_HUNDRED).normalize())
}

/// `dd.mm`: the month header carries the month and year.
fn day(date: NaiveDate) -> DisplayText {
    DisplayText::plain(date.format("%d.%m").to_string())
}

/// Every conversion failure in crates/core (`CalculateDividendTaxError`,
/// `CalculateInterestTaxError`, `CalculateSellBuyValuesError`) starts like
/// this. One missing NBP rate fails the whole calculation and blocks the page.
/// Below this table width, Import in the header shows only its icon.
const NARROW: f32 = 600.;

/// The records of one month that match the filter, by index.
struct MonthGroup {
    key: (i32, u32),
    title: SharedString,
    records: Vec<usize>,
    summary: Vec<(Option<&'static str>, DisplayText)>,
}

/// An open add or edit dialog.
struct Editor<K: RecordKind> {
    form: K::Form,
    /// Whether the optional ID is unfolded.
    id_open: bool,
    /// The values the shown or pending preview is for.
    draft: Option<Draft<K>>,
    /// Kept while a newer preview calculates, so the band doesn't flicker.
    preview: Option<Result<Preview, SharedString>>,
    /// Replacing this drops an older preview's result.
    preview_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

pub struct RecordsPage<K: RecordKind> {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    records: Vec<K::Record>,
    totals: Vec<(&'static str, DisplayText)>,
    columns: Vec<RecordColumn>,
    groups: Vec<MonthGroup>,
    filter: Entity<InputState>,
    /// The filter in lower case, trimmed.
    query: String,
    folded: HashSet<(i32, u32)>,
    table_scroll: ScrollHandle,
    page_scroll: ScrollHandle,
    table_state: RecordTableState,
    selected: HashSet<String>,
    expanded: HashSet<String>,
    editor: Option<Editor<K>>,
    pending_delete: Option<Vec<String>>,
    nbp_year: Option<Entity<InputState>>,
    /// The last load's conversion error, while a missing rate blocks the page.
    missing_rate: Option<SharedString>,
    /// For the empty state: the nearest year with this page's records.
    other_year: Option<(i32, u32)>,
    focus: FocusHandle,
    _subscriptions: [Subscription; 2],
}

impl<K: RecordKind> RecordsPage<K> {
    pub fn new(
        context: PageContext,
        year: Option<i32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter"));
        let filter_subscription = cx.subscribe_in(&filter, window, |this, filter, event, _, cx| {
            if let InputEvent::Change = event {
                let query = filter.read(cx).value().trim().to_lowercase();
                if query != this.query {
                    this.query = query;
                    this.regroup();
                    this.table_state.scroll_to_top();
                    cx.notify();
                }
            }
        });
        let focus_subscription = cx.on_focus_lost(window, |this, window, cx| {
            // When a focused virtual row scrolls out of view, keep focus on
            // the page instead of dropping it.
            if this.editor.is_none()
                && this.pending_delete.is_none()
                && this.nbp_year.is_none()
                && window.focus_lost_restore_target(cx).as_ref() == Some(&this.focus)
            {
                window.focus(&this.focus, cx);
            }
        });
        let mut view = Self {
            context,
            year,
            status: Status::default(),
            records: Vec::new(),
            totals: Vec::new(),
            columns: K::columns(),
            groups: Vec::new(),
            filter,
            query: String::new(),
            folded: HashSet::new(),
            table_scroll: ScrollHandle::default(),
            page_scroll: ScrollHandle::default(),
            table_state: RecordTableState::default(),
            selected: HashSet::new(),
            expanded: HashSet::new(),
            editor: None,
            pending_delete: None,
            nbp_year: None,
            missing_rate: None,
            other_year: None,
            focus: cx.focus_handle(),
            _subscriptions: [filter_subscription, focus_subscription],
        };
        view.refresh(window, cx);
        view
    }

    fn disabled(&self) -> bool {
        self.status.busy
            || self.status.loading
            || self.editor.is_some()
            || self.pending_delete.is_some()
            || self.nbp_year.is_some()
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(
            self.status.busy
                || self.editor.is_some()
                || self.pending_delete.is_some()
                || self.nbp_year.is_some(),
            cx,
        );
        cx.notify();
    }

    /// Takes a successful load. Selection and opened rows keep the records
    /// that are still there.
    fn apply(
        &mut self,
        totals: Vec<Decimal>,
        records: Vec<K::Record>,
        other_year: Option<(i32, u32)>,
    ) {
        self.totals = K::TOTAL_LABELS
            .iter()
            .copied()
            .zip(totals.into_iter().map(pln))
            .collect();
        self.other_year = other_year;
        let rows = records.iter().map(K::display).collect::<Vec<_>>();
        let changed = rows != self.table_state.rows;
        if changed {
            let ids = records.iter().map(K::id).collect::<HashSet<_>>();
            self.selected.retain(|id| ids.contains(id.as_str()));
            self.expanded.retain(|id| ids.contains(id.as_str()));
            self.table_state.reset(rows);
        }
        self.records = records;
        if changed {
            self.regroup();
        }
    }

    /// Groups the records that match the filter by month, with subtotals.
    fn regroup(&mut self) {
        let mut groups: Vec<MonthGroup> = Vec::new();
        let rows = self.records.iter().zip(&self.table_state.rows);
        for (index, (record, row)) in rows.enumerate() {
            if !row.search.contains(self.query.as_str()) {
                continue;
            }
            let date = K::date(record);
            let key = (date.year(), date.month());
            match groups.last_mut() {
                Some(group) if group.key == key => group.records.push(index),
                _ => groups.push(MonthGroup {
                    key,
                    title: month_title(key, self.year.is_none()),
                    records: vec![index],
                    summary: vec![],
                }),
            }
        }
        for group in &mut groups {
            let records = group
                .records
                .iter()
                .map(|&index| &self.records[index])
                .collect::<Vec<_>>();
            group.summary = K::subtotal(&records)
                .into_iter()
                .map(|(label, value)| (label, pln(value)))
                .collect();
        }
        self.groups = groups;
        self.table_state.dirty = true;
    }

    /// The records that match the filter.
    fn visible(&self) -> impl Iterator<Item = &str> {
        self.groups
            .iter()
            .flat_map(|group| &group.records)
            .map(|&index| K::id(&self.records[index]))
    }

    fn open_editor(&mut self, form: K::Form, window: &mut Window, cx: &mut Context<Self>) {
        let first_input = form.first_input().focus_handle(cx);
        let subscriptions = form.watch(window, cx, Self::update_preview);
        self.editor = Some(Editor {
            form,
            id_open: false,
            draft: None,
            preview: None,
            preview_task: None,
            _subscriptions: subscriptions,
        });
        self.status.error = None;
        self.status.message = None;
        dialog::open(window, cx, Self::render_editor);
        window.focus(&first_input, cx);
        self.update_preview(window, cx);
        self.notify(cx);
    }

    /// Recalculates the preview when a value it reads changes; an invalid
    /// value hides it. Only the latest calculation's result is shown.
    fn update_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let draft = editor.form.draft(cx);
        if draft == editor.draft {
            return;
        }
        let task = draft.clone().map(|draft| {
            self.context.services.run(
                window,
                cx,
                move |app| K::preview(app, draft),
                |this, result, _, cx| {
                    if let Some(editor) = &mut this.editor {
                        editor.preview = Some(
                            result
                                .map(|record| K::preview_display(&record))
                                .map_err(SharedString::from),
                        );
                        cx.notify();
                    }
                },
            )
        });
        if let Some(editor) = &mut self.editor {
            if draft.is_none() {
                editor.preview = None;
            }
            editor.draft = draft;
            editor.preview_task = task;
        }
        cx.notify();
    }

    /// Unfolds or folds the optional ID. Unfolding it while adding focuses it.
    fn toggle_id(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        editor.id_open = !editor.id_open;
        if editor.id_open && editor.form.existing_id().is_none() {
            let id = editor.form.id_input().focus_handle(cx);
            window.focus(&id, cx);
        }
        cx.notify();
    }

    fn edit(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        if let Some(record) = self.records.iter().find(|record| K::id(record) == id) {
            let form = K::Form::new(Some(record), window, cx);
            self.open_editor(form, window, cx);
        }
    }

    fn close_editor(&mut self, cx: &mut Context<Self>) {
        self.editor = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(editor) = &self.editor else {
            return;
        };
        let submission = match editor.form.submission(cx) {
            Ok(submission) => submission,
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
                return;
            }
        };
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                K::save(app, submission).await?;
                Ok("Record saved.".into())
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    this.context.years_changed(cx);
                    window.close_dialog(cx);
                    this.close_editor(cx);
                    window.focus(&this.focus, cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(ids) = self.pending_delete.take() else {
            return;
        };
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let count = K::delete(app, ids).await?;
                Ok(format!("Deleted {count} record(s)."))
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    this.context.years_changed(cx);
                    window.focus(&this.focus, cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn confirm_delete(
        &mut self,
        ids: Vec<String>,
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled() {
            return;
        }
        self.pending_delete = Some(ids);
        dialog::confirm(
            "Delete records",
            message,
            "Delete",
            Self::delete,
            |this, cx| {
                this.pending_delete = None;
                this.notify(cx);
            },
            window,
            cx,
        );
        self.notify(cx);
    }

    fn delete_one(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_delete(
            vec![id.to_owned()],
            "Delete this record? This cannot be undone.".into(),
            window,
            cx,
        );
    }

    /// Deletes every selected record, including any the filter hides.
    fn delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let visible = self.visible().collect::<HashSet<_>>();
        let hidden = self
            .selected
            .iter()
            .filter(|id| !visible.contains(id.as_str()))
            .count();
        let message = if hidden > 0 {
            format!(
                "Delete {} selected record(s)? {hidden} of them are hidden by the filter. \
                 This cannot be undone.",
                self.selected.len()
            )
        } else {
            format!(
                "Delete {} selected record(s)? This cannot be undone.",
                self.selected.len()
            )
        };
        let ids = self.selected.iter().cloned().collect();
        self.confirm_delete(ids, message, window, cx);
    }

    fn render_editor(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(editor) = &self.editor else {
            return dialog;
        };
        let busy = self.status.busy;
        let p = *palette(cx);
        let body = v_flex()
            .pt(px(8.))
            .child(editor.form.render(busy, cx))
            .child(self.id_field(editor, busy, cx).mt(px(14.)))
            .when_some(self.status.error.clone(), |body, error| {
                body.child(
                    h_flex()
                        .mt(px(12.))
                        .items_start()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(p.danger)
                        .child(
                            Icon::new(IconName::CircleX)
                                .size(px(14.))
                                .flex_shrink_0()
                                .mt(px(1.)),
                        )
                        .child(div().min_w_0().child(error)),
                )
            })
            .when_some(editor.preview.as_ref(), |body, preview| {
                body.child(preview_band(preview, cx).mt(px(16.)))
            });
        dialog::editor(
            self,
            dialog,
            body,
            if busy { "Saving…" } else { "Save" },
            |this| &this.status,
            Self::save,
            Self::close_editor,
            cx,
        )
        .title(div().text_size(px(15.)).child(editor.form.title()))
        .w(px(540.))
        .bg(p.popover)
        .border_color(p.strong_line)
        .px(px(20.))
        .pb(px(14.))
    }

    /// The optional ID, folded under "ID · optional, generated if left
    /// blank". When editing, the line shows the record's ID instead, and the
    /// field can't be changed.
    fn id_field(&self, editor: &Editor<K>, busy: bool, cx: &mut Context<Self>) -> Collapsible {
        let p = *palette(cx);
        let existing = editor.form.existing_id();
        let note = match existing {
            Some(id) => format!("· {id}"),
            None => "· optional, generated if left blank".into(),
        };
        Collapsible::new()
            .open(editor.id_open)
            .gap(px(10.))
            .child(
                h_flex().child(
                    Button::new("editor-id")
                        .ghost()
                        .h(px(22.))
                        .px(px(4.))
                        .ml(px(-4.))
                        .gap(px(6.))
                        .rounded(px(6.))
                        .text_color(p.muted)
                        .accessibility_label("ID")
                        .child(
                            Icon::new(if editor.id_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .size(px(12.)),
                        )
                        // The button sizes its own text, so the line sets 12px
                        // on a child.
                        .child(
                            h_flex()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .child("ID")
                                .child(div().text_color(p.faint).child(note)),
                        )
                        .on_click(cx.listener(|this, _, window, cx| this.toggle_id(window, cx))),
                ),
            )
            .content(form::input_field(
                "ID",
                editor.form.id_input(),
                busy || existing.is_some(),
                cx,
            ))
    }

    /// The Rates page's "Import from NBP" dialog, prefilled with the year.
    fn open_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        let year = self.year.unwrap_or_else(|| chrono::Local::now().year());
        let input = nbp::year_input(year, window, cx);
        self.nbp_year = Some(input.clone());
        self.status.error = None;
        self.status.message = None;
        dialog::open(window, cx, Self::nbp_dialog);
        window.focus(&input.focus_handle(cx), cx);
        self.notify(cx);
    }

    fn close_nbp(&mut self, cx: &mut Context<Self>) {
        self.nbp_year = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn import_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(input) = &self.nbp_year else {
            return;
        };
        let year = match nbp::year(input, cx) {
            Ok(year) => year,
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
                return;
            }
        };
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let count = rate::import_api(&app, year).await?;
                Ok(format!("Imported {count} rates."))
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    window.close_dialog(cx);
                    this.close_nbp(cx);
                    window.focus(&this.focus, cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn nbp_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(input) = &self.nbp_year else {
            return dialog;
        };
        nbp::dialog(
            self,
            dialog,
            input,
            |this| &this.status,
            Self::import_nbp,
            Self::close_nbp,
            cx,
        )
    }

    fn header(&self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let disabled = self.disabled();
        let p = *palette(cx);
        let filterable = self.status.ready && !self.records.is_empty();
        let shown = self
            .groups
            .iter()
            .map(|group| group.records.len())
            .sum::<usize>();
        // A narrow panel, such as the minimum window with the sidebar
        // expanded, has no room for the filter count beside Import's label.
        let narrow = self
            .table_state
            .viewport_width
            .is_some_and(|width| width < px(NARROW));
        let count = if narrow {
            format!("{shown} of {}", self.records.len())
        } else {
            format!("Showing {shown} of {}", self.records.len())
        };
        let import = if narrow {
            header::icon_button("open-imports", IconName::Upload, "Import")
        } else {
            header::button("open-imports", IconName::Upload, "Import")
        };
        header::page(
            K::PAGE.title(),
            Some(header::year_label(self.year)),
            window,
            cx,
        )
        .when(filterable, |row| {
            row.child(
                header::no_drag()
                    .flex()
                    .flex_shrink(1.)
                    .min_w_0()
                    .items_center()
                    .gap(px(8.))
                    .when(!self.query.is_empty(), |field| {
                        field.child(
                            div()
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .font_features(tabular_digits())
                                .child(count),
                        )
                    })
                    .child(
                        div()
                            .flex_basis(px(180.))
                            .flex_shrink(1.)
                            .min_w(px(110.))
                            .child(
                                Input::new(&self.filter)
                                    .small()
                                    .h(px(28.))
                                    .rounded(px(8.))
                                    .aria_label("Filter records")
                                    .prefix(
                                        Icon::new(IconName::Search)
                                            .size(px(14.))
                                            .text_color(p.faint),
                                    )
                                    .cleanable(true)
                                    .disabled(self.editor.is_some() || self.nbp_year.is_some()),
                            ),
                    ),
            )
        })
        .child(
            header::actions()
                .child(self.status.refreshing(cx))
                .child(header::refresh(disabled, cx).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.context.years_changed(cx);
                        this.refresh(window, cx);
                    },
                )))
                .child(import.disabled(disabled).on_click(
                    cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx)),
                ))
                .child(
                    self.add_button("add-record", Some(IconName::Plus), disabled, cx)
                        .primary(),
                ),
        )
    }

    fn add_button(
        &self,
        id: &'static str,
        icon: Option<IconName>,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        header::button(id, icon, "Add new")
            .disabled(disabled)
            .on_click(cx.listener(|this, _, window, cx| {
                if !this.disabled() {
                    let form = K::Form::new(None, window, cx);
                    this.open_editor(form, window, cx);
                }
            }))
    }

    /// The year totals with copy buttons. A filter doesn't change them.
    fn totals(&self, compact: bool, cx: &App) -> Div {
        h_flex()
            .flex_shrink_0()
            .flex_wrap()
            .items_end()
            .gap_x(px(if compact { 24. } else { 40. }))
            .gap_y(px(8.))
            .px(px(20.))
            .pt(px(4.))
            .pb(px(14.))
            .children(self.totals.iter().map(|(label, value)| {
                value::stat(
                    SharedString::from(format!("total-{label}")),
                    *label,
                    value,
                    cx,
                )
            }))
    }

    fn loading(&self, cx: &App) -> Div {
        let visible = self.status.loading_visible;
        let layout = TableLayout::new(
            &self.columns,
            self.table_state.viewport_width.unwrap_or(px(900.)),
        );
        v_flex()
            .flex_1()
            .min_h_0()
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_x(px(40.))
                    .px(px(20.))
                    .pt(px(4.))
                    .pb(px(14.))
                    .children(
                        K::TOTAL_LABELS
                            .iter()
                            .map(|label| value::stat_skeleton(label, visible, cx)),
                    ),
            )
            .child(record_skeleton(&self.columns, &layout, visible, cx))
    }

    /// No records for the year: Import, Add new, and a link to the nearest
    /// year that has this page's records.
    fn empty_state(&self, cx: &mut Context<Self>) -> Empty {
        let disabled = self.disabled();
        let title = match self.year {
            Some(year) => format!("No {} in {year}", K::PLURAL),
            None => format!("No {} yet", K::PLURAL),
        };
        let providers = IMPORTERS
            .iter()
            .filter(|importer| importer.output.iter().any(K::imported))
            .map(|importer| importer.name)
            .collect::<Vec<_>>()
            .join(" or ");
        let description = format!("Import a {providers} statement, or add a record by hand.");
        let link = self.other_year.map(|(year, count)| {
            let records = if count == 1 { "record" } else { "records" };
            Button::new("other-year")
                .ghost()
                .small()
                .text_color(cx.theme().muted_foreground)
                .disabled(disabled)
                .child(div().text_size(px(12.)).child(format!(
                    "{year} has {count} {} {records} · switch to {year}",
                    K::NAME.to_lowercase()
                )))
                .child(Icon::new(IconName::ChevronRight).size(px(12.)))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.context.select_year(Some(year), cx)),
                )
        });
        notice::notice(K::PAGE.icon(), false, title, description, cx).content(
            EmptyContent::new()
                .child(
                    h_flex()
                        .gap(px(8.))
                        .mt(px(6.))
                        .child(
                            header::button("empty-import", None, "Import")
                                .primary()
                                .disabled(disabled)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.context.navigate(Page::Imports, cx)
                                })),
                        )
                        .child(self.add_button("empty-add", None, disabled, cx)),
                )
                .children(link),
        )
    }

    fn missing_rate_state(&self, error: &SharedString, cx: &mut Context<Self>) -> Empty {
        notice::missing_rate(
            self.year,
            error.clone(),
            self.disabled(),
            cx.listener(|this, _, window, cx| this.open_nbp(window, cx)),
            cx.listener(|this, _, _, cx| this.context.navigate(Page::Rates, cx)),
            cx,
        )
    }

    /// Floats over the table while anything is selected. The count includes
    /// records the filter hides.
    fn selection_bar(&self, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let disabled = self.disabled();
        let danger = if disabled { p.muted } else { p.danger };
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
                            .child(format!("{} selected", self.selected.len())),
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
                            .child(Icon::new(IconName::Trash).size(px(14.)).text_color(danger))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_medium()
                                    .text_color(danger)
                                    .child("Remove selected"),
                            )
                            .on_click(
                                cx.listener(|this, _, window, cx| this.delete_selected(window, cx)),
                            ),
                    )
                    .child(
                        header::icon_button("clear-selection", IconName::X, "Clear selection")
                            .text_color(p.muted)
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.selected.clear();
                                cx.notify();
                            })),
                    ),
            )
    }
}

fn month_title((year, month): (i32, u32), with_year: bool) -> SharedString {
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

/// The year with this page's records nearest to `selected`, newer first on a
/// tie. Nothing for all years, which already includes every record.
fn other_year<K: RecordKind>(selected: Option<i32>, years: &[YearInfo]) -> Option<(i32, u32)> {
    let selected = selected?;
    years
        .iter()
        .filter(|info| info.year != selected && K::count(info) > 0)
        .min_by_key(|info| ((info.year - selected).abs(), -info.year))
        .map(|info| (info.year, K::count(info)))
}

impl<K: RecordKind> PageView for RecordsPage<K> {
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
        let year = self.year;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let (totals, records) = K::load(app.clone(), year).await?;
                let other_year = if records.is_empty() {
                    let years = year::list_year_info(&app).await.unwrap_or_default();
                    other_year::<K>(year, &years)
                } else {
                    None
                };
                Ok((totals, records, other_year))
            },
            |this, result, _, cx| {
                let loaded = this.status.loaded(result);
                this.missing_rate = this
                    .status
                    .error
                    .take_if(|error| missing_rate(error).is_some());
                if let Some((totals, records, other_year)) = loaded {
                    this.apply(totals, records, other_year);
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl<K: RecordKind> Render for RecordsPage<K> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = v_flex()
            .id("record-page")
            .track_focus(&self.focus)
            .tab_index(-1)
            .size_full()
            .min_h_0()
            // overflow_y_scrollbar() wraps its content in h_auto(), which
            // unbounds the virtual list below.
            .overflow_y_scroll()
            .lock_scroll_axis()
            .track_scroll(&self.page_scroll)
            // The editor shows its own errors.
            .when(self.status.is_visible() && self.editor.is_none(), |view| {
                view.child(
                    div()
                        .flex_shrink_0()
                        .px(px(20.))
                        .pb_3()
                        .child(self.status.render()),
                )
            });
        if let Some(error) = self.missing_rate.clone() {
            body = body.child(self.missing_rate_state(&error, cx));
        } else if self.status.ready && self.records.is_empty() {
            body = body.child(self.empty_state(cx));
        } else if self.status.ready {
            let compact = self.table_state.layout.as_ref().is_some_and(|l| l.compact);
            body = body
                .child(self.totals(compact, cx))
                .child(self.table(window, cx));
        } else if self.status.loading {
            body = body.child(self.loading(cx));
        } else {
            let disabled = self.status.busy || self.pending_delete.is_some();
            body = body.child(
                h_flex().px(px(20.)).gap_2().child(
                    Button::new("retry")
                        .label("Retry")
                        .outline()
                        .disabled(disabled)
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            );
        }
        let selecting = self.status.ready && !self.selected.is_empty();
        v_flex()
            .size_full()
            .min_h_0()
            .child(self.header(window, cx))
            .child(
                v_flex()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(body)
                    .vertical_scrollbar(&self.page_scroll)
                    .when(selecting, |view| view.child(self.selection_bar(cx))),
            )
    }
}
