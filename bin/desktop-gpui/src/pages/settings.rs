use super::PageView;
use crate::{
    components::{
        self, Status,
        form::{self, Choice, ChoiceState},
        header,
    },
    navigation::{Page, PageContext},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        button::*,
        group_box::{GroupBox, GroupBoxVariants},
        *,
    },
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
        if self.status.busy || self.status.loading {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = components::page_content()
            .gap_4()
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render())
            });
        if let Some(dividend_rounding) = &self.dividend_rounding {
            content = content.child(
                GroupBox::new()
                    .id("dividend-rounding-settings")
                    .fill()
                    .content_style(StyleRefinement::default().p_5().gap_4())
                    .child(components::section_heading("Calculation settings"))
                    .child(
                        h_flex()
                            .w_full()
                            .items_end()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                form::select_field(
                                    "Dividend rounding",
                                    dividend_rounding,
                                    self.status.busy || self.status.loading,
                                    cx,
                                )
                                .flex_1()
                                .min_w(px(240.)),
                            )
                            .child(
                                Button::new("save-settings")
                                    .flex_shrink_0()
                                    .label(if self.status.busy {
                                        "Saving…"
                                    } else {
                                        "Save changes"
                                    })
                                    .primary()
                                    .disabled(
                                        self.status.busy
                                            || self.status.loading
                                            || !self.has_changes(cx),
                                    )
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.save(window, cx)),
                                    ),
                            ),
                    )
                    .when(self.has_changes(cx) && !self.status.busy, |view| {
                        view.child(
                            div()
                                .text_base()
                                .text_color(cx.theme().muted_foreground)
                                .child("Unsaved changes"),
                        )
                    }),
            );
        } else if self.status.loading {
            content = content.child(
                GroupBox::new()
                    .id("dividend-rounding-loading")
                    .fill()
                    .content_style(StyleRefinement::default().p_5().gap_4())
                    .child(components::section_heading("Calculation settings"))
                    .child(div().text_base().font_medium().child("Dividend rounding"))
                    .child(
                        div()
                            .h_8()
                            .w_full()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().skeleton)
                            .opacity(if self.status.loading_visible { 1. } else { 0. }),
                    ),
            );
        }
        if self.status.error.is_some() && !self.status.loading {
            content = content.child(
                h_flex().child(
                    Button::new("retry")
                        .label("Retry")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            );
        }
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                header::page(Page::Settings.title(), None, window, cx)
                    .child(header::actions().child(self.status.refreshing(cx))),
            )
            .child(components::scroll(content))
    }
}
