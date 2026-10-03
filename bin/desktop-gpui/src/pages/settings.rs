use super::PageView;
use crate::{
    components::{
        self, Status,
        form::{self, Choice, ChoiceState},
    },
    navigation::PageContext,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{button::*, *},
    *,
};
use pitpls_app::use_case::settings;
use pitpls_core::settings::{DividendRounding, Settings};

pub struct SettingsPage {
    context: PageContext,
    status: Status,
    saved_rounding: Option<DividendRounding>,
    selection_subscription: Option<Subscription>,
    dividend_rounding: Option<Entity<ChoiceState<DividendRounding>>>,
}

impl SettingsPage {
    pub fn new(context: PageContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            context,
            status: Status::default(),
            dividend_rounding: None,
            saved_rounding: None,
            selection_subscription: None,
        };
        page.refresh(window, cx);
        page
    }

    fn has_changes(&self, cx: &App) -> bool {
        self.dividend_rounding
            .as_ref()
            .and_then(|state| state.read(cx).selected_value().copied())
            .is_some_and(|value| Some(value) != self.saved_rounding)
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading || !self.has_changes(cx) {
            return;
        }
        let Some(dividend_rounding) = &self.dividend_rounding else {
            return;
        };
        let dividend_rounding = match form::selected(dividend_rounding, "Dividend rounding", cx) {
            Ok(value) => value,
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
                return;
            }
        };
        let settings = Settings { dividend_rounding };
        self.status.begin_save();
        self.context.set_locked(true, cx);
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                settings::update_settings(&app, settings).await?;
                Ok("Settings saved.".into())
            },
            move |this, result, _, cx| {
                if this.status.saved(result) {
                    this.saved_rounding = Some(dividend_rounding);
                }
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
        self.dividend_rounding = None;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            |app| async move { settings::load_settings(&app).await },
            |this, result, window, cx| {
                if let Some(settings) = this.status.loaded(result) {
                    this.saved_rounding = Some(settings.dividend_rounding);
                    this.dividend_rounding = Some(form::select(
                        vec![
                            Choice::new(DividendRounding::SumToGroszy, "Sum to groszy"),
                            Choice::new(DividendRounding::SumToPayToZlote, "Sum to pay to złote"),
                            Choice::new(DividendRounding::SumBothToZlote, "Sum both to złote"),
                            Choice::new(DividendRounding::AllToZlote, "All to złote"),
                        ],
                        settings.dividend_rounding,
                        window,
                        cx,
                    ));
                    this.selection_subscription = this.dividend_rounding.as_ref().map(|state| {
                        cx.subscribe(
                            state,
                            |this,
                             _,
                             _: &select::SelectEvent<Vec<form::Choice<DividendRounding>>>,
                             cx| {
                                this.status.message = None;
                                cx.notify();
                            },
                        )
                    });
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = v_flex()
            .gap_4()
            .p_5()
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render(cx))
            });
        if let Some(dividend_rounding) = &self.dividend_rounding {
            content = content
                .child(
                    div()
                        .text_lg()
                        .font_semibold()
                        .child("Calculation settings"),
                )
                .child(form::select_field(
                    "Dividend rounding",
                    dividend_rounding,
                    self.status.busy,
                    cx,
                ))
                .child(
                    h_flex().child(
                        Button::new("save-settings")
                            .label(if self.status.busy {
                                "Saving…"
                            } else {
                                "Save"
                            })
                            .primary()
                            .disabled(self.status.busy || !self.has_changes(cx))
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
