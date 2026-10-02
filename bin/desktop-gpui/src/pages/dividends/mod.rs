mod form;
mod table;

use self::form::{DividendForm, DividendSubmission};
use super::PageView;
use crate::{
    components::{self, Status, SummaryGroup},
    format::pln,
    navigation::{Page, PageContext},
};
use gpui_kit::{
    component::{button::*, *},
    *,
};
use pitpls_app::use_case::dividend;
use pitpls_core::dividend::CalculatedDividend;
use std::collections::HashSet;

struct Confirmation {
    message: SharedString,
    ids: Vec<String>,
}

pub struct DividendsPage {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    records: Vec<CalculatedDividend>,
    summaries: Vec<SummaryGroup>,
    table_scroll: ScrollHandle,
    selected: HashSet<String>,
    expanded: HashSet<String>,
    editor: Option<DividendForm>,
    confirmation: Option<Confirmation>,
    return_focus: Option<FocusHandle>,
}

impl DividendsPage {
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
            selected: HashSet::new(),
            expanded: HashSet::new(),
            editor: None,
            confirmation: None,
            return_focus: None,
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

    fn open_editor(&mut self, form: DividendForm, window: &mut Window, cx: &mut Context<Self>) {
        self.return_focus = window.focused(cx);
        form.focus(window, cx);
        self.editor = Some(form);
        self.status.error = None;
        self.status.message = None;
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
                    DividendSubmission::Create(input) => {
                        dividend::create_dividend(&app, input).await?;
                    }
                    DividendSubmission::Update(input) => {
                        dividend::update_dividend(&app, input).await?;
                    }
                }
                Ok("Record saved.".into())
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    this.close_editor(window, cx);
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
                let count = dividend::delete_dividends(&app, confirmation.ids).await?;
                Ok(format!("Deleted {count} record(s)."))
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn actions(&self, cx: &mut Context<Self>) -> Div {
        let disabled = self.status.busy || self.status.loading || self.confirmation.is_some();
        h_flex()
            .flex_wrap()
            .gap_2()
            .child(
                Button::new("add-record")
                    .label("Add record")
                    .primary()
                    .disabled(disabled)
                    .on_click(cx.listener(|this, _, window, cx| {
                        let form = DividendForm::new(None, window, cx);
                        this.open_editor(form, window, cx);
                    })),
            )
            .child(
                Button::new("delete-selected")
                    .label(format!("Delete selected ({})", self.selected.len()))
                    .outline()
                    .disabled(disabled || self.selected.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirmation = Some(Confirmation {
                            message: format!(
                                "Delete {} selected record(s)? This cannot be undone.",
                                this.selected.len()
                            )
                            .into(),
                            ids: this.selected.iter().cloned().collect(),
                        });
                        this.notify(cx);
                    })),
            )
    }

    fn render_editor(&self, editor: &DividendForm, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap_5()
            .p_5()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(div().text_lg().font_semibold().child(editor.title()))
            .child(editor.render(self.status.busy, cx))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("cancel-editor")
                            .label("Cancel")
                            .outline()
                            .disabled(self.status.busy)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.close_editor(window, cx)),
                            ),
                    )
                    .child(
                        Button::new("save-editor")
                            .label("Save")
                            .primary()
                            .disabled(self.status.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
            )
    }
}

impl PageView for DividendsPage {
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
            move |app| async move { dividend::load_dividends(&app, year).await },
            |this, result, _, cx| {
                if let Some(data) = this.status.loaded(result) {
                    this.summaries = vec![SummaryGroup {
                        title: "Dividend totals",
                        values: vec![
                            ("Income", pln(data.income)),
                            ("Calculated tax", pln(data.to_pay)),
                            ("Creditable foreign tax", pln(data.paid)),
                        ],
                    }];
                    this.records = data.calculated;
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for DividendsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = v_flex().gap_4().p_5().child(self.status.render(cx));
        if let Some(editor) = &self.editor {
            content = content.child(self.render_editor(editor, cx));
        } else {
            content = content
                .child(self.actions(cx))
                .child(components::period(self.year, cx));
            if let Some(confirmation) = &self.confirmation {
                content = content.child(
                    v_flex()
                        .gap_3()
                        .p_4()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius)
                        .child(confirmation.message.clone())
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new("cancel-delete")
                                        .label("Cancel")
                                        .outline()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.confirmation = None;
                                            this.notify(cx);
                                        })),
                                )
                                .child(
                                    Button::new("confirm-delete")
                                        .label("Confirm")
                                        .primary()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.delete(window, cx)
                                        })),
                                ),
                        ),
                );
            }
            if self.status.error.is_some() || (!self.status.ready && !self.status.loading) {
                content = content.child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("retry")
                                .label("Retry")
                                .outline()
                                .disabled(self.status.busy || self.confirmation.is_some())
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.refresh(window, cx)),
                                ),
                        )
                        .child(
                            Button::new("open-rates")
                                .label("Open rates")
                                .outline()
                                .disabled(self.status.busy || self.confirmation.is_some())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.context.navigate(Page::Rates, cx)
                                })),
                        ),
                );
            }
            if self.status.ready {
                content = content
                    .child(components::summaries(&self.summaries, cx))
                    .child(self.records(cx));
            }
        }
        components::scroll(content)
    }
}
