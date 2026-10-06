mod crypto;
mod dividends;
mod interests;
mod table;

pub use self::{crypto::Crypto, dividends::Dividends, interests::Interests};

use super::PageView;
use crate::{
    components::{
        self, Status, SummaryGroup, dialog,
        records::{RecordTableState, RowDisplay},
        table::Column,
    },
    format::pln,
    navigation::{Page, PageContext},
};
use chrono::NaiveDate;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{button::*, dialog::Dialog, input::InputState, scroll::ScrollableElement, *},
    *,
};
use rust_decimal::Decimal;
use std::{collections::HashSet, future::Future, sync::Arc};

pub enum Submission<C, U> {
    Create(C),
    Update(U),
}

pub trait RecordForm: 'static {
    type Record;
    type Submission: Send + 'static;

    fn new(record: Option<&Self::Record>, window: &mut Window, cx: &mut App) -> Self;
    fn title(&self) -> &'static str;
    /// The input focused when the editor opens. Enter opens a focused date
    /// picker, so this is a text input, where Enter saves.
    fn first_input(&self) -> &Entity<InputState>;
    fn submission(&self, cx: &App) -> Result<Self::Submission, String>;
    fn render(&self, busy: bool, cx: &App) -> Div;
}

pub trait RecordKind: 'static {
    type Record: Send + 'static;
    type Form: RecordForm<Record = Self::Record>;

    /// Names a single record in accessibility labels.
    const NAME: &'static str;
    const TOTALS_TITLE: &'static str;
    const TOTAL_LABELS: &'static [&'static str];

    fn id(record: &Self::Record) -> &str;
    fn date(record: &Self::Record) -> NaiveDate;
    fn columns() -> Vec<Column>;
    fn display(record: &Self::Record) -> RowDisplay;

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
}

pub struct RecordsPage<K: RecordKind> {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    records: Vec<K::Record>,
    summaries: Vec<SummaryGroup>,
    table_scroll: ScrollHandle,
    page_scroll: ScrollHandle,
    table_state: RecordTableState,
    selected: HashSet<String>,
    expanded: HashSet<String>,
    editor: Option<K::Form>,
    pending_delete: Option<Vec<String>>,
    focus: FocusHandle,
    _focus_subscription: Subscription,
}

impl<K: RecordKind> RecordsPage<K> {
    pub fn new(
        context: PageContext,
        year: Option<i32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            context,
            year,
            status: Status::default(),
            records: Vec::new(),
            summaries: Vec::new(),
            table_scroll: ScrollHandle::default(),
            page_scroll: ScrollHandle::default(),
            table_state: RecordTableState::default(),
            selected: HashSet::new(),
            expanded: HashSet::new(),
            editor: None,
            pending_delete: None,
            focus: cx.focus_handle(),
            _focus_subscription: cx.on_focus_lost(window, |this, window, cx| {
                // When a focused virtual row scrolls out of view, keep focus
                // on the page instead of dropping it.
                if this.editor.is_none()
                    && this.pending_delete.is_none()
                    && window.focus_lost_restore_target(cx).as_ref() == Some(&this.focus)
                {
                    window.focus(&this.focus, cx);
                }
            }),
        };
        view.refresh(window, cx);
        view
    }

    fn disabled(&self) -> bool {
        self.status.busy
            || self.status.loading
            || self.editor.is_some()
            || self.pending_delete.is_some()
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(
            self.status.busy || self.editor.is_some() || self.pending_delete.is_some(),
            cx,
        );
        cx.notify();
    }

    fn open_editor(&mut self, form: K::Form, window: &mut Window, cx: &mut Context<Self>) {
        let first_input = form.first_input().focus_handle(cx);
        self.editor = Some(form);
        self.status.error = None;
        self.status.message = None;
        dialog::open(window, cx, Self::render_editor);
        window.focus(&first_input, cx);
        self.notify(cx);
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
        let submission = match editor.submission(cx) {
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
                    window.focus(&this.focus, cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn actions(&self, cx: &mut Context<Self>) -> Div {
        let disabled = self.disabled();
        h_flex()
            .w_full()
            .flex_shrink_0()
            .flex_wrap()
            .justify_between()
            .gap_2()
            .child(
                Button::new("add-record")
                    .label("Add new")
                    .primary()
                    .disabled(disabled)
                    .on_click(cx.listener(|this, _, window, cx| {
                        let form = K::Form::new(None, window, cx);
                        this.open_editor(form, window, cx);
                    })),
            )
            .child(
                h_flex()
                    .gap_2()
                    .when(!self.expanded.is_empty(), |row| {
                        row.child(
                            Button::new("collapse-all")
                                .label("Collapse all")
                                .outline()
                                .disabled(disabled)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.expanded.clear();
                                    this.table_state.dirty = true;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        Button::new("delete-selected")
                            .label("Remove selected")
                            .danger()
                            .disabled(disabled || self.selected.is_empty())
                            .on_click(cx.listener(|this, _, window, cx| {
                                let message = format!(
                                    "Delete {} selected record(s)? This cannot be undone.",
                                    this.selected.len()
                                );
                                let ids = this.selected.iter().cloned().collect();
                                this.confirm_delete(ids, message, window, cx);
                            })),
                    ),
            )
    }

    fn confirm_delete(
        &mut self,
        ids: Vec<String>,
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn render_editor(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(editor) = &self.editor else {
            return dialog;
        };
        let busy = self.status.busy;
        dialog::form(
            self,
            dialog,
            if busy { "Saving…" } else { "Save" },
            |this| &this.status,
            Self::save,
            Self::close_editor,
            cx,
        )
        .title(editor.title())
        .w(px(740.))
        .max_w(px(740.))
        .child(editor.render(busy, cx))
    }
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
            move |app| K::load(app, year),
            |this, result, _, cx| {
                if let Some((totals, records)) = this.status.loaded(result) {
                    this.summaries = vec![SummaryGroup {
                        title: K::TOTALS_TITLE,
                        values: K::TOTAL_LABELS
                            .iter()
                            .copied()
                            .zip(totals.into_iter().map(pln))
                            .collect(),
                    }];
                    let rows = records.iter().map(K::display).collect::<Vec<_>>();
                    if rows != this.table_state.rows {
                        this.selected.clear();
                        this.expanded.clear();
                        this.table_state.reset(rows);
                    }
                    this.records = records;
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl<K: RecordKind> Render for RecordsPage<K> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = components::page_content()
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
            .gap_4()
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render())
            });
        if self.status.error.is_some() || (!self.status.ready && !self.status.loading) {
            let disabled = self.status.busy || self.pending_delete.is_some();
            content = content.child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("retry")
                            .label("Retry")
                            .outline()
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                    )
                    .child(
                        Button::new("open-rates")
                            .label("Open rates")
                            .outline()
                            .disabled(disabled)
                            .on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.context.navigate(Page::Rates, cx)
                                }),
                            ),
                    ),
            );
        }
        if self.status.ready {
            content = content
                .child(components::summaries(&self.summaries, cx))
                .child(self.actions(cx))
                .child(self.records(window, cx));
        } else if self.status.loading {
            content = content
                .child(components::summary_skeleton(
                    K::TOTALS_TITLE,
                    K::TOTAL_LABELS,
                    self.status.loading_visible,
                    cx,
                ))
                .child(self.actions(cx))
                .child(components::records::record_skeleton(
                    K::columns(),
                    self.status.loading_visible,
                    cx,
                ));
        } else {
            content = content.child(self.actions(cx));
        }
        div()
            .relative()
            .size_full()
            .min_h_0()
            .child(content)
            .child(self.status.refreshing(cx))
            .vertical_scrollbar(&self.page_scroll)
    }
}
