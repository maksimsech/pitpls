use super::PageView;
use crate::{
    components::{
        self, Status, SummaryGroup,
        form::Form,
        table::{Column, cell},
    },
    navigation::{Page, PageContext},
};
use gpui_kit::{
    assets::IconName,
    component::{button::*, checkbox::Checkbox, *},
    *,
};
use serde_json::{Value, json};
use std::{collections::HashSet, future::Future, marker::PhantomData};
mod table;

const SELECT_WIDTH: f32 = 42.;
const ACTION_WIDTH: f32 = 132.;

pub trait RecordDefinition: 'static {
    const PAGE: Page;

    fn load(
        app: &pitpls_app::App,
        year: Option<i32>,
    ) -> impl Future<Output = Result<RecordData, String>> + Send;

    fn save(
        app: &pitpls_app::App,
        editing: bool,
        values: Value,
    ) -> impl Future<Output = Result<String, String>> + Send;

    fn delete(
        app: &pitpls_app::App,
        ids: Vec<String>,
    ) -> impl Future<Output = Result<String, String>> + Send;

    fn form(existing: Option<Value>, window: &mut Window, cx: &mut App) -> Form;
}

pub struct RecordRow {
    pub id: String,
    pub cells: Vec<SharedString>,
    pub details: Vec<(SharedString, SharedString)>,
    pub edit: Value,
}

#[derive(Default)]
pub struct RecordData {
    pub columns: Vec<Column>,
    pub rows: Vec<RecordRow>,
    pub summaries: Vec<SummaryGroup>,
}

struct RecordEditor {
    form: Form,
    editing: bool,
}
struct Confirmation {
    message: SharedString,
    ids: Vec<String>,
}

pub struct RecordsPage<D> {
    context: PageContext,
    year: Option<i32>,
    status: Status,
    data: RecordData,
    table_scroll: ScrollHandle,
    selected: HashSet<String>,
    expanded: HashSet<String>,
    editor: Option<RecordEditor>,
    confirmation: Option<Confirmation>,
    return_focus: Option<FocusHandle>,
    definition: PhantomData<D>,
}

impl<D: RecordDefinition> RecordsPage<D> {
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
            data: RecordData::default(),
            table_scroll: ScrollHandle::default(),
            selected: HashSet::new(),
            expanded: HashSet::new(),
            editor: None,
            confirmation: None,
            return_focus: None,
            definition: PhantomData,
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

    fn open_editor(
        &mut self,
        form: Form,
        editing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.return_focus = window.focused(cx);
        form.focus(window, cx);
        self.editor = Some(RecordEditor { form, editing });
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
        let editing = editor.editing;
        let mut values = match editor.form.values(cx) {
            Ok(values) => values,
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
                return;
            }
        };
        if !editing && values["id"] == "" {
            values["id"] = Value::Null;
        }
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move { D::save(&app, editing, values).await },
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
            move |app| async move { D::delete(&app, confirmation.ids).await },
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
                        let form = D::form(None, window, cx);
                        this.open_editor(form, false, window, cx);
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

    fn render_editor(&self, editor: &RecordEditor, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap_5()
            .p_5()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .child(editor.form.title.clone()),
            )
            .child(editor.form.render(self.status.busy, cx))
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

impl<D: RecordDefinition> PageView for RecordsPage<D> {
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
            move |app| async move { D::load(&app, year).await },
            |this, result, _, cx| {
                if let Some(data) = this.status.loaded(result) {
                    this.data = data;
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl<D: RecordDefinition> Render for RecordsPage<D> {
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
                    .child(components::summaries(&self.data.summaries, cx))
                    .child(self.records(cx));
            }
        }
        components::scroll(content)
    }
}

pub fn edit_base(
    id: &str,
    date: chrono::NaiveDate,
    value: pitpls_core::common::Amount,
    provider: &str,
) -> Value {
    json!({"id": id, "date": date.to_string(), "value": value.value.to_string(),
        "value_currency": value.currency, "provider": provider})
}

pub fn record_form(
    name: &str,
    existing: Option<Value>,
    defaults: Value,
    window: &mut Window,
    cx: &mut App,
) -> (Form, Value) {
    let editing = existing.is_some();
    let mut values = existing.unwrap_or(defaults);
    if !editing {
        values["id"] = json!("");
        values["date"] = json!(chrono::Local::now().date_naive().to_string());
    }
    let mut form = Form::new(format!("{} {name}", if editing { "Edit" } else { "Add" }));
    form.input(
        "id",
        if editing {
            "ID"
        } else {
            "ID (optional; generated when blank)"
        },
        &values,
        editing,
        window,
        cx,
    );
    form.input("date", "Date (YYYY-MM-DD)", &values, false, window, cx);
    (form, values)
}
