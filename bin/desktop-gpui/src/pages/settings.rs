use super::PageView;
use crate::{
    components::{
        self, Status,
        form::{Choice, Form},
    },
    navigation::PageContext,
};
use gpui_kit::{
    component::{button::*, *},
    *,
};
use pitpls_app::use_case::settings;
use pitpls_core::settings::Settings;
use serde_json::json;

pub struct SettingsPage {
    context: PageContext,
    status: Status,
    form: Option<Form>,
}

impl SettingsPage {
    pub fn new(context: PageContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            context,
            status: Status::default(),
            form: None,
        };
        page.refresh(window, cx);
        page
    }

    fn form(settings: Settings, window: &mut Window, cx: &mut App) -> Form {
        let mut form = Form::new("Calculation settings");
        form.choice(
            "dividend_rounding",
            "Dividend rounding",
            vec![
                Choice::new("SumToGroszy", "Sum to groszy"),
                Choice::new("SumToPayToZlote", "Sum to pay to złote"),
                Choice::new("SumBothToZlote", "Sum both to złote"),
                Choice::new("AllToZlote", "All to złote"),
            ],
            &json!(settings),
            window,
            cx,
        );
        form
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading {
            return;
        }
        let Some(form) = &self.form else {
            return;
        };
        let settings = match form
            .values(cx)
            .and_then(|value| serde_json::from_value::<Settings>(value).map_err(|e| e.to_string()))
        {
            Ok(value) => value,
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
                return;
            }
        };
        self.status.begin_save();
        self.context.set_locked(true, cx);
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                settings::update_settings(&app, settings).await?;
                Ok("Settings saved.".into())
            },
            |this, result, _, cx| {
                this.status.saved(result);
                this.context.set_locked(false, cx);
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl PageView for SettingsPage {
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        self.status.begin_load();
        self.form = None;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            |app| async move { settings::load_settings(&app).await },
            |this, result, window, cx| {
                if let Some(settings) = this.status.loaded(result) {
                    this.form = Some(Self::form(settings, window, cx));
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = v_flex().gap_4().p_5().child(self.status.render(cx));
        if let Some(form) = &self.form {
            content = content
                .child(div().text_lg().font_semibold().child(form.title.clone()))
                .child(form.render(self.status.busy, cx))
                .child(
                    h_flex().child(
                        Button::new("save-settings")
                            .label("Save settings")
                            .primary()
                            .disabled(self.status.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
                );
        } else if self.status.error.is_some() {
            content = content.child(
                h_flex().child(
                    Button::new("retry")
                        .label("Retry")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            );
        }
        components::scroll(content)
    }
}
