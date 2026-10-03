use gpui_kit::prelude::FluentBuilder;
mod form;
mod table;

use self::form::{InterestForm, InterestSubmission};
use super::PageView;
use crate::{
    components::{self, Status, SummaryGroup},
    format::pln,
    navigation::{Page, PageContext},
};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{
    component::{button::*, *},
    *,
};
use pitpls_app::use_case::interest;
use pitpls_core::interest::CalculatedInterest;
use std::collections::HashSet;

struct Confirmation {
    message: SharedString,
    ids: Vec<String>,
}

pub struct InterestsPage {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    records: Vec<CalculatedInterest>,
    summaries: Vec<SummaryGroup>,
    table_scroll: ScrollHandle,
    page_scroll: ScrollHandle,
    table_state: components::records::RecordTableState,
    selected: HashSet<String>,
    expanded: HashSet<String>,
    editor: Option<InterestForm>,
    confirmation: Option<Confirmation>,
    return_focus: Option<FocusHandle>,
    focus: FocusHandle,
    _focus_subscription: Subscription,
}

impl InterestsPage {
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
            table_state: components::records::RecordTableState::default(),
            selected: HashSet::new(),
            expanded: HashSet::new(),
            editor: None,
            confirmation: None,
            return_focus: None,
            focus: cx.focus_handle(),
            _focus_subscription: cx.on_focus_lost(window, |this, window, cx| {
                // GPUI supplies the nearest surviving focus ancestor when a
                // virtual row is unmounted, including scrollbar/keyboard scroll.
                if this.editor.is_none()
                    && this.confirmation.is_none()
                    && window.focus_lost_restore_target(cx).as_ref() == Some(&this.focus)
                {
                    window.focus(&this.focus, cx);
                }
            }),
        };
        view.refresh(window, cx);
        view
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(
            self.status.busy || self.editor.is_some() || self.confirmation.is_some(),
            cx,
        );
        cx.notify();
    }

    fn open_editor(&mut self, form: InterestForm, window: &mut Window, cx: &mut Context<Self>) {
        self.return_focus = window.focused(cx);
        self.editor = Some(form);
        self.status.error = None;
        self.status.message = None;
        let page = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            page.update(cx, |this, cx| this.render_editor(dialog, cx))
                .unwrap_or_else(|_| Dialog::new(cx))
        });
        if let Some(editor) = &self.editor {
            editor.focus(window, cx);
        }
        self.notify(cx);
    }

    fn close_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor = None;
        self.status.error = None;
        if let Some(focus) = self.return_focus.take() {
            window.focus(&focus, cx);
        }
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
            Ok(input) => input,
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
                match submission {
                    InterestSubmission::Create(input) => {
                        interest::create_interest(&app, input).await?;
                    }
                    InterestSubmission::Update(input) => {
                        interest::update_interest(&app, input).await?;
                    }
                }
                Ok("Record saved.".into())
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    window.close_dialog(cx);
                    this.close_editor(window, cx);
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
        let Some(confirmation) = self.confirmation.take() else {
            return;
        };
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let count = interest::delete_interests(&app, confirmation.ids).await?;
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
        let disabled = self.status.busy
            || self.status.loading
            || self.editor.is_some()
            || self.confirmation.is_some();
        h_flex()
            .flex_shrink_0()
            .flex_wrap()
            .gap_2()
            .child(
                Button::new("add-record")
                    .label("Add new")
                    .primary()
                    .disabled(disabled)
                    .on_click(cx.listener(|this, _, window, cx| {
                        let form = InterestForm::new(None, window, cx);
                        this.open_editor(form, window, cx);
                    })),
            )
            .child(
                Button::new("delete-selected")
                    .label("Remove selected")
                    .danger()
                    .disabled(disabled || self.selected.is_empty())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_confirmation(
                            Confirmation {
                                message: format!(
                                    "Delete {} selected record(s)? This cannot be undone.",
                                    this.selected.len()
                                )
                                .into(),
                                ids: this.selected.iter().cloned().collect(),
                            },
                            window,
                            cx,
                        );
                    })),
            )
    }

    fn open_confirmation(
        &mut self,
        confirmation: Confirmation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirmation = Some(confirmation);
        let page = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            page.update(cx, |this, cx| {
                let confirm = cx.entity().downgrade();
                dialog
                    .title("Delete records")
                    .footer(components::confirmation_footer("Delete"))
                    .child(
                        this.confirmation
                            .as_ref()
                            .map(|value| value.message.clone())
                            .unwrap_or_default(),
                    )
                    .on_ok(move |_, window, cx| {
                        confirm
                            .update(cx, |this, cx| this.delete(window, cx))
                            .is_ok()
                    })
                    .on_close(cx.listener(|this, _, _, cx| {
                        this.confirmation = None;
                        this.notify(cx);
                    }))
            })
            .unwrap_or_else(|_| Dialog::new(cx))
        });
        self.notify(cx);
    }

    fn render_editor(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(editor) = &self.editor else {
            return dialog;
        };
        let dismiss = cx.entity().downgrade();
        let submit = cx.entity().downgrade();
        dialog
            .title(editor.title())
            .w(px(740.))
            .max_w(px(740.))
            .overlay_closable(!self.status.busy)
            .keyboard(!self.status.busy)
            .close_button(!self.status.busy)
            .on_ok(move |_, window, cx| {
                let _ = submit.update(cx, |this, cx| this.save(window, cx));
                false // Close only after the asynchronous write succeeds.
            })
            .on_cancel(move |_, _, cx| {
                dismiss
                    .update(cx, |this, _| !this.status.busy)
                    .unwrap_or(true)
            })
            .on_close(cx.listener(|this, _, window, cx| {
                if !this.status.busy {
                    this.close_editor(window, cx);
                }
            }))
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render(cx))
            })
            .child(editor.render(self.status.busy, cx))
            .footer(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-editor")
                            .label("Cancel")
                            .outline()
                            .disabled(self.status.busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if !this.status.busy {
                                    window.close_dialog(cx);
                                    this.close_editor(window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("save-editor")
                            .label(if self.status.busy {
                                "Saving…"
                            } else {
                                "Save"
                            })
                            .primary()
                            .disabled(self.status.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
            )
    }
}

impl PageView for InterestsPage {
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.editor.is_some() || self.confirmation.is_some() {
            return;
        }
        self.status.begin_load();
        self.selected.clear();
        self.expanded.clear();
        let year = self.year;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move { interest::load_interests(&app, year).await },
            |this, result, _, cx| {
                if let Some(data) = this.status.loaded(result) {
                    this.summaries = vec![SummaryGroup {
                        title: "Interest totals",
                        values: vec![
                            ("Income (I-65)", pln(data.income)),
                            ("To pay (G-47)", pln(data.to_pay)),
                        ],
                    }];
                    this.records = data.calculated;
                    this.table_state
                        .reset(this.records.iter().map(Self::display_record).collect());
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for InterestsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = v_flex()
            .id("record-page")
            .track_focus(&self.focus)
            .tab_index(-1)
            .size_full()
            .min_h_0()
            // Keep this flex viewport bounded. overflow_y_scrollbar() wraps
            // its content in h_auto(), which is unsuitable around a virtual list.
            .overflow_y_scroll()
            .lock_scroll_axis()
            .track_scroll(&self.page_scroll)
            .gap_4()
            .p_5()
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render(cx))
            });
        content = content.child(self.actions(cx));
        if self.status.error.is_some() || (!self.status.ready && !self.status.loading) {
            content = content.child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("retry")
                            .label("Retry")
                            .outline()
                            .disabled(self.status.busy || self.confirmation.is_some())
                            .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                    )
                    .child(
                        Button::new("open-rates")
                            .label("Open rates")
                            .outline()
                            .disabled(self.status.busy || self.confirmation.is_some())
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
                .child(self.records(window, cx));
        }
        div()
            .relative()
            .size_full()
            .min_h_0()
            .child(content)
            .vertical_scrollbar(&self.page_scroll)
            .into_any_element()
    }
}
