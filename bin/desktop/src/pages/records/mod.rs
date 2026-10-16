mod crypto;
mod dividends;
mod interests;
mod table;

pub use self::{
    crypto::Crypto,
    dividends::Dividends,
    interests::Interests,
    table::{RecordTable, TableHost, selection_bar, table},
};

use crate::{
    components::{
        ButtonText, Status, dialog, form, header, nbp, notice,
        records::{Preview, RecordColumn, RowDisplay, preview_band},
        value,
    },
    format::{DisplayText, amount, date, pln, record_count},
    i18n::{or_list, plural, plural_key},
    navigation::{Page, PageContext},
    services::Error as ServiceError,
    theme::{palette, tabular_digits},
};
use chrono::{DateTime, Datelike, NaiveTime, Utc};
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
use pitpls_app::use_case::year::{self, YearInfo};
use pitpls_core::{common::Amount, time};
use pitpls_importers::{IMPORTERS, OutputType};
use rust_decimal::Decimal;
use rust_i18n::t;
use std::{borrow::Cow, collections::HashSet, future::Future, sync::Arc};

pub enum Submission<C, U> {
    Create(C),
    Update(U),
}

pub trait RecordForm: 'static {
    type Record;
    type Submission: Send + 'static;
    /// The values the calculation reads, so the preview recalculates only when
    /// they change.
    type Draft: Clone + PartialEq + Send + 'static;

    fn new(record: Option<&Self::Record>, window: &mut Window, cx: &mut App) -> Self;
    fn title(&self) -> Cow<'static, str>;
    fn existing_id(&self) -> Option<&str>;
    fn id_input(&self) -> &Entity<InputState>;
    /// A text input, not the date picker: Enter opens a focused date picker
    /// instead of saving.
    fn first_input(&self) -> &Entity<InputState>;
    fn submission(&self, cx: &App) -> Result<Self::Submission, String>;
    fn draft(&self, cx: &App) -> Option<Self::Draft>;
    fn watch<V: 'static>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
        changed: fn(&mut V, &mut Window, &mut Context<V>),
    ) -> Vec<Subscription>;
    fn render(&self, busy: bool, cx: &App) -> Div;
}

type Draft<K> = <<K as RecordKind>::Form as RecordForm>::Draft;

pub trait RecordKind: 'static {
    type Record: Send + 'static;
    type Form: RecordForm<Record = Self::Record>;

    const PAGE: Page;
    /// Message keys, as are the labels below; `OTHER_YEAR` takes a count.
    const EMPTY: &'static str;
    const EMPTY_YEAR: &'static str;
    const OTHER_YEAR: &'static str;
    const TOTAL_LABELS: &'static [&'static str];

    fn id(record: &Self::Record) -> &str;
    fn columns() -> Vec<RecordColumn>;
    fn display(record: &Self::Record) -> RowDisplay;
    /// Information only: summed here at full precision, never by the
    /// calculation.
    fn subtotal(records: &[&Self::Record]) -> Vec<(Option<&'static str>, Decimal)>;
    fn imported(output: &OutputType) -> bool;
    fn count(year: &YearInfo) -> u32;

    /// The totals come in `TOTAL_LABELS` order.
    fn load(
        app: Arc<pitpls_app::App>,
        year: Option<i32>,
    ) -> impl Future<Output = Result<(Vec<Decimal>, Vec<Self::Record>), ServiceError>> + Send;
    fn save(
        app: Arc<pitpls_app::App>,
        submission: <Self::Form as RecordForm>::Submission,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send;
    fn delete(
        app: Arc<pitpls_app::App>,
        ids: Vec<String>,
    ) -> impl Future<Output = Result<u64, ServiceError>> + Send;
    fn preview(
        app: Arc<pitpls_app::App>,
        draft: Draft<Self>,
    ) -> impl Future<Output = Result<Self::Record, ServiceError>> + Send;
    fn preview_display(record: &Self::Record) -> Preview;
}

fn conversion(value: Amount, rate: Decimal) -> Vec<DisplayText> {
    vec![amount(value), DisplayText::plain(format!("× {rate}"))]
}

fn percent(rate: Decimal) -> String {
    format!("{}%", (rate * Decimal::ONE_HUNDRED).normalize())
}

/// The month header carries the month and year. Midnight is left out, since
/// date-only sources record it.
pub fn day(value: DateTime<Utc>) -> DisplayText {
    let local = time::local(value);
    let clock = if local.time() == NaiveTime::MIN {
        String::new()
    } else {
        local.format("%H:%M").to_string()
    };
    DisplayText {
        extra: clock.into(),
        ..DisplayText::plain(local.format("%d.%m").to_string())
    }
}

pub fn date_label(value: DateTime<Utc>) -> String {
    date(time::tax_date(value)).main.to_string()
}

const NARROW: f32 = 600.;

struct Editor<K: RecordKind> {
    form: K::Form,
    id_open: bool,
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
    table: RecordTable,
    filter: Entity<InputState>,
    query: String,
    page_scroll: ScrollHandle,
    editor: Option<Editor<K>>,
    pending_delete: Option<Vec<String>>,
    deleting: HashSet<String>,
    nbp_year: Option<Entity<InputState>>,
    missing_rate: Option<SharedString>,
    other_year: Option<(i32, u32)>,
    focus: FocusHandle,
    _subscriptions: [Subscription; 2],
}

impl<K: RecordKind> RecordsPage<K> {
    pub fn new(
        context: PageContext,
        year: Option<i32>,
        width: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(t!("records.filter")));
        let filter_subscription = cx.subscribe_in(&filter, window, |this, filter, event, _, cx| {
            if let InputEvent::Change = event {
                let query = filter.read(cx).value().trim().to_lowercase();
                if query != this.query {
                    this.query = query;
                    this.regroup();
                    this.table.state.scroll_to_top();
                    cx.notify();
                }
            }
        });
        let focus_subscription = cx.on_focus_lost(window, |this, window, cx| {
            // Runs after the shell's fallback. Focus that falls back to the
            // page or above it stays on the page: when a focused virtual row
            // scrolls out of view, or when a deleted record's Delete button,
            // which its confirmation hands focus back to, is gone.
            let fallback = window.focus_lost_restore_target(cx);
            if !window.has_active_dialog(cx)
                && fallback.is_none_or(|target| {
                    target == this.focus || target.contains(&this.focus, window)
                })
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
            table: RecordTable::new("records-horizontal", K::columns(), true, width),
            filter,
            query: String::new(),
            page_scroll: ScrollHandle::default(),
            editor: None,
            pending_delete: None,
            deleting: HashSet::new(),
            nbp_year: None,
            missing_rate: None,
            other_year: None,
            focus: cx.focus_handle(),
            _subscriptions: [filter_subscription, focus_subscription],
        };
        view.refresh(window, cx);
        view
    }

    fn locked(&self) -> bool {
        self.status.busy
            || self.editor.is_some()
            || self.pending_delete.is_some()
            || self.nbp_year.is_some()
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(self.locked(), cx);
        cx.notify();
    }

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
        let changed = self
            .table
            .set_rows(records.iter().map(K::display).collect());
        self.records = records;
        if changed {
            self.regroup();
        }
    }

    fn regroup(&mut self) {
        let mut groups = self.table.month_groups(&self.query, self.year.is_none());
        for group in &mut groups {
            let records = group
                .records
                .iter()
                .map(|&index| &self.records[index])
                .collect::<Vec<_>>();
            group.count = record_count(records.len() as u32).into();
            group.summary = K::subtotal(&records)
                .into_iter()
                .map(|(label, value)| (label, pln(value)))
                .collect();
        }
        self.table.set_groups(groups);
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
                Ok(t!("records.saved").into_owned())
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
        self.deleting = ids.iter().cloned().collect();
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let count = K::delete(app, ids).await?;
                Ok(plural("records.deleted", count).into_owned())
            },
            |this, result, window, cx| {
                this.deleting.clear();
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
            t!("records.delete_title"),
            message,
            t!("common.delete"),
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

    /// Includes the selected records the filter hides.
    fn delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = &self.table.selected;
        let message = delete_selected_message(selected, self.table.visible());
        let ids = selected.iter().cloned().collect();
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
            .when_some(self.status.error.as_ref(), |body, error| {
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
                        .child(div().min_w_0().child(error.text())),
                )
            })
            .when_some(editor.preview.as_ref(), |body, preview| {
                body.child(preview_band(preview, cx).mt(px(16.)))
            });
        dialog::editor(
            self,
            dialog,
            body,
            if busy {
                t!("common.saving")
            } else {
                t!("common.save")
            },
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

    fn id_field(&self, editor: &Editor<K>, busy: bool, cx: &mut Context<Self>) -> Collapsible {
        let p = *palette(cx);
        let existing = editor.form.existing_id();
        let note = match existing {
            Some(id) => format!("· {id}"),
            None => t!("records.id_note").into_owned(),
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
                        .accessibility_label(t!("field.id"))
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
                                .child(t!("field.id"))
                                .child(div().text_color(p.faint).child(note)),
                        )
                        .on_click(cx.listener(|this, _, window, cx| this.toggle_id(window, cx))),
                ),
            )
            .content(form::input_field(
                t!("field.id"),
                editor.form.id_input(),
                busy || existing.is_some(),
                cx,
            ))
    }

    fn open_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        let year = self.year.unwrap_or_else(|| chrono::Local::now().year());
        self.nbp_year = Some(nbp::open(year, Self::nbp_dialog, window, cx));
        self.status.error = None;
        self.status.message = None;
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
            move |app| nbp::import(app, year),
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
        let filterable = self.status.ready && !self.records.is_empty();
        let filter_disabled = self.editor.is_some() || self.nbp_year.is_some();
        let shown = self.table.shown();
        let narrow = self
            .table
            .state
            .viewport_width
            .is_some_and(|width| width < px(NARROW));
        let count =
            (!self.query.is_empty()).then(|| filter_count(shown, self.records.len(), narrow));
        let import = import_button(narrow);
        header::page(
            K::PAGE.title(),
            Some(header::year_label(self.year)),
            window,
            cx,
        )
        .when(filterable, |row| {
            row.child(filter_field(&self.filter, count, filter_disabled, cx))
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
        header::button(id, icon, t!("records.add"))
            .disabled(disabled)
            .on_click(cx.listener(|this, _, window, cx| {
                if !this.disabled() {
                    let form = K::Form::new(None, window, cx);
                    this.open_editor(form, window, cx);
                }
            }))
    }

    fn totals(&self, compact: bool, cx: &App) -> Div {
        totals_row(compact).children(self.totals.iter().map(|(label, value)| {
            value::stat(
                SharedString::from(format!("total-{label}")),
                t!(*label),
                value,
                cx,
            )
        }))
    }

    fn loading(&self, cx: &App) -> Div {
        let visible = self.status.loading_visible;
        let (skeleton, compact) = self.table.skeleton(visible, cx);
        v_flex()
            .flex_1()
            .min_h_0()
            .child(
                totals_row(compact).children(
                    K::TOTAL_LABELS
                        .iter()
                        .map(|label| value::stat_skeleton(t!(*label), visible, cx)),
                ),
            )
            .child(skeleton)
    }

    fn empty_state(&self, cx: &mut Context<Self>) -> Empty {
        let disabled = self.disabled();
        let title = match self.year {
            Some(year) => t!(K::EMPTY_YEAR, year = year),
            None => t!(K::EMPTY),
        };
        let providers = IMPORTERS
            .iter()
            .filter(|importer| importer.output.iter().any(K::imported))
            .map(|importer| crate::messages::importer_name(importer.kind).to_owned())
            .collect::<Vec<_>>();
        let description = t!("records.empty_detail", providers = or_list(&providers));
        let link = self.other_year.map(|(year, count)| {
            let key = plural_key(K::OTHER_YEAR, count.into());
            Button::new("other-year")
                .ghost()
                .small()
                .text_color(cx.theme().muted_foreground)
                .disabled(disabled)
                .child(
                    div()
                        .text_size(px(12.))
                        .child(t!(key, year = year, count = count)),
                )
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
                            header::button("empty-import", None, t!("common.import"))
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
}

impl<K: RecordKind> TableHost for RecordsPage<K> {
    type Table = ();

    fn table(&self, _: ()) -> &RecordTable {
        &self.table
    }

    fn table_mut(&mut self, _: ()) -> &mut RecordTable {
        &mut self.table
    }

    fn disabled(&self) -> bool {
        self.locked() || self.status.loading
    }

    fn deleting(&self) -> &HashSet<String> {
        &self.deleting
    }

    fn page_focus(&self) -> &FocusHandle {
        &self.focus
    }

    fn editable(&self) -> bool {
        true
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

    fn delete_one(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_delete(
            vec![id.to_owned()],
            t!("records.delete_one").into_owned(),
            window,
            cx,
        );
    }
}

pub fn delete_selected_message<'a>(
    selected: &HashSet<String>,
    visible: impl Iterator<Item = &'a str>,
) -> String {
    let visible = visible.collect::<HashSet<_>>();
    let hidden = selected
        .iter()
        .filter(|id| !visible.contains(id.as_str()))
        .count();
    let question = plural("records.delete_selected", selected.len() as u64);
    let undo = t!("records.cannot_undo");
    if hidden > 0 {
        let hidden = plural("records.hidden_by_filter", hidden as u64);
        format!("{question} {hidden} {undo}")
    } else {
        format!("{question} {undo}")
    }
}

/// A narrow panel, such as the minimum window with the sidebar expanded, has
/// no room for "Showing" beside Import's label.
pub fn filter_count(shown: usize, total: usize, narrow: bool) -> String {
    if narrow {
        t!("records.shown_short", shown = shown, total = total).into_owned()
    } else {
        t!("records.shown", shown = shown, total = total).into_owned()
    }
}

pub fn import_button(narrow: bool) -> Button {
    if narrow {
        header::icon_button("open-imports", IconName::Upload, t!("common.import"))
    } else {
        header::button("open-imports", IconName::Upload, t!("common.import"))
    }
}

pub fn filter_field(
    filter: &Entity<InputState>,
    count: Option<String>,
    disabled: bool,
    cx: &App,
) -> Div {
    let p = *palette(cx);
    header::no_drag()
        .flex()
        .flex_shrink(1.)
        .min_w_0()
        .items_center()
        .gap(px(8.))
        .children(count.map(|count| {
            div()
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_size(px(12.))
                .text_color(p.faint)
                .font_features(tabular_digits())
                .child(count)
        }))
        .child(
            // A width, not a flex basis: the row sizes itself from its
            // content, so a basis left the field at its minimum.
            div().w(px(180.)).flex_shrink(1.).min_w(px(110.)).child(
                Input::new(filter)
                    .small()
                    // `Input::h` only sizes multi-line inputs.
                    .map(|input| Styled::h(input, px(28.)))
                    .pl(px(11.))
                    .gap_2()
                    .text_size(px(13.))
                    .rounded(px(8.))
                    // The buttons' fill; disabled keeps the input's, close
                    // to a disabled button's.
                    .when(!disabled, |input| input.bg(cx.theme().tokens.button))
                    .aria_label(t!("records.filter_label"))
                    .prefix(
                        Icon::new(IconName::Search)
                            .size(px(14.))
                            .text_color(p.faint),
                    )
                    .cleanable(true)
                    .disabled(disabled),
            ),
        )
}

pub fn no_match(query: &str, cx: &App) -> Div {
    div()
        .px(px(20.))
        .py(px(24.))
        .text_color(palette(cx).muted)
        .child(t!("records.no_match", query = query))
}

pub fn totals_row(compact: bool) -> Div {
    h_flex()
        .flex_shrink_0()
        .flex_wrap()
        .items_end()
        .gap_x(px(if compact { 24. } else { 40. }))
        .gap_y(px(8.))
        .px(px(20.))
        .pt(px(4.))
        .pb(px(14.))
}

/// Newer first on a tie. Nothing for all years, which already includes every
/// record.
fn other_year<K: RecordKind>(selected: Option<i32>, years: &[YearInfo]) -> Option<(i32, u32)> {
    let selected = selected?;
    years
        .iter()
        .filter(|info| info.year != selected && K::count(info) > 0)
        .min_by_key(|info| ((info.year - selected).abs(), -info.year))
        .map(|info| (info.year, K::count(info)))
}

impl<K: RecordKind> RecordsPage<K> {
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
                let missing_rate = result
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.missing_rate());
                let loaded = this.status.loaded(result);
                this.missing_rate = if missing_rate {
                    this.status.error.take().map(|error| error.text())
                } else {
                    None
                };
                if let Some((totals, records, other_year)) = loaded {
                    this.apply(totals, records, other_year);
                }
                this.context.loaded(cx);
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
            // The editor and the NBP dialog show their own errors.
            .when(
                self.status.is_visible() && self.editor.is_none() && self.nbp_year.is_none(),
                |view| {
                    view.child(
                        div()
                            .flex_shrink_0()
                            .px(px(20.))
                            .pb_3()
                            .child(self.status.render(|this| &mut this.status, cx)),
                    )
                },
            );
        if let Some(error) = self.missing_rate.clone() {
            body = body.child(self.missing_rate_state(&error, cx));
        } else if self.status.ready && self.records.is_empty() {
            body = body.child(self.empty_state(cx));
        } else if self.status.ready {
            let compact = self.table.compact();
            let empty = no_match(&self.query, cx);
            body = body
                .child(self.totals(compact, cx))
                .child(table(self, (), empty, window, cx));
        } else if self.status.loading {
            body = body.child(self.loading(cx));
        } else {
            let disabled = self.status.busy || self.pending_delete.is_some();
            body = body.child(
                h_flex().px(px(20.)).gap_2().child(
                    Button::new("retry")
                        .text_label(t!("common.retry"))
                        .outline()
                        .disabled(disabled)
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            );
        }
        let selecting = self.status.ready && !self.table.selected.is_empty();
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
                    .when(selecting, |view| {
                        view.child(selection_bar(self, (), Self::delete_selected, cx))
                    }),
            )
    }
}
