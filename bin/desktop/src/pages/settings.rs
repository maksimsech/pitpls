use crate::{
    components::{self, ButtonText, Status, data, header},
    i18n::{Language, language_choice},
    navigation::{Page, PageContext},
    theme::{palette, theme_choice},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        alert::Alert,
        button::*,
        radio::{Radio, RadioGroup},
        *,
    },
    *,
};
use pitpls_app::use_case::settings;
use pitpls_core::settings::{DividendRounding, Settings};
use rust_i18n::t;
use std::borrow::Cow;

/// Titles and details are message keys.
const ROUNDINGS: [(DividendRounding, &str, &str); 4] = [
    (
        DividendRounding::SumToGroszy,
        "settings.rounding_groszy",
        "settings.rounding_groszy_detail",
    ),
    (
        DividendRounding::SumToPayToZlote,
        "settings.rounding_to_pay",
        "settings.rounding_to_pay_detail",
    ),
    (
        DividendRounding::SumBothToZlote,
        "settings.rounding_both",
        "settings.rounding_both_detail",
    ),
    (
        DividendRounding::AllToZlote,
        "settings.rounding_all",
        "settings.rounding_all_detail",
    ),
];

const THEMES: [Option<bool>; 3] = [None, Some(false), Some(true)];

const LANGUAGES: [Option<Language>; 3] = [None, Some(Language::English), Some(Language::Polish)];

fn theme_label(dark: Option<bool>) -> Cow<'static, str> {
    match dark {
        None => t!("settings.system"),
        Some(false) => t!("settings.light"),
        Some(true) => t!("settings.dark"),
    }
}

fn language_label(language: Option<Language>) -> Cow<'static, str> {
    language.map_or_else(|| t!("settings.system"), |language| language.name().into())
}

pub struct SettingsPage {
    context: PageContext,
    status: Status,
    saved: Option<DividendRounding>,
    picked: Option<DividendRounding>,
    theme_focus: [FocusHandle; 3],
    language_focus: [FocusHandle; 3],
    /// Around the rounding options, so the arrow keys stay among them.
    rounding_focus: FocusHandle,
}

impl SettingsPage {
    pub fn new(context: PageContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            context,
            status: Status::default(),
            saved: None,
            picked: None,
            theme_focus: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle()],
            language_focus: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle()],
            rounding_focus: cx.focus_handle(),
        };
        page.refresh(window, cx);
        page
    }

    fn has_changes(&self) -> bool {
        self.picked.is_some() && self.picked != self.saved
    }

    fn pick(&mut self, rounding: DividendRounding, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading {
            return;
        }
        self.picked = Some(rounding);
        self.status.error = None;
        cx.notify();
    }

    fn discard(&mut self, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        self.picked = self.saved;
        self.status.error = None;
        cx.notify();
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading || !self.has_changes() {
            return;
        }
        let Some(dividend_rounding) = self.picked else {
            return;
        };
        let settings = Settings { dividend_rounding };
        self.status.begin_save();
        self.context.set_locked(true, cx);
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move { Ok(settings::update_settings(&app, settings).await?) },
            move |this, result, _, cx| {
                this.status.busy = false;
                match result {
                    Ok(()) => this.saved = Some(dividend_rounding),
                    Err(error) => this.status.error = Some(error.into()),
                }
                this.context.set_locked(false, cx);
                cx.notify();
            },
        ));
        cx.notify();
    }

    /// Up and Down move focus without picking an option. A step out of the
    /// group steps back, so focus stops at the ends.
    fn step_rounding(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let forward = match event.keystroke.key.as_str() {
            "down" => true,
            "up" => false,
            _ => return,
        };
        if event.keystroke.modifiers.modified() {
            return;
        }
        cx.stop_propagation();
        let step = |forward: bool, window: &mut Window, cx: &mut App| {
            if forward {
                window.focus_next(cx);
            } else {
                window.focus_prev(cx);
            }
        };
        step(forward, window, cx);
        if !self.rounding_focus.contains_focused(window, cx) {
            step(!forward, window, cx);
        }
    }

    fn step_segment(
        focus: &[FocusHandle],
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let step: isize = match event.keystroke.key.as_str() {
            "right" => 1,
            "left" => -1,
            _ => return,
        };
        if event.keystroke.modifiers.modified() {
            return;
        }
        let Some(index) = focus.iter().position(|focus| focus.is_focused(window)) else {
            return;
        };
        cx.stop_propagation();
        let next = index.saturating_add_signed(step).min(focus.len() - 1);
        window.focus(&focus[next], cx);
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            |app| async move { Ok(settings::load_settings(&app).await?) },
            |this, result, _, cx| {
                if let Some(settings) = this.status.loaded(result) {
                    this.saved = Some(settings.dividend_rounding);
                    this.picked = Some(settings.dividend_rounding);
                }
                this.context.loaded(cx);
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for SettingsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .text_size(px(13.))
            .child(header::page(Page::Settings.title(), None, window, cx))
            .child(components::scroll(
                div().w_full().px(px(20.)).child(
                    v_flex()
                        .w_full()
                        .max_w(px(640.))
                        .mx_auto()
                        .pt(px(6.))
                        .pb(px(32.))
                        .gap(px(16.))
                        .child(self.rounding_card(cx))
                        .child(self.appearance_card(window, cx)),
                ),
            ))
    }
}

impl SettingsPage {
    fn rounding_card(&self, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        // Not clipped, so a focused option's ring shows.
        let card = data::open_card(cx).child(
            v_flex()
                .gap(px(2.))
                .pt(px(14.))
                .pb(px(10.))
                .px(px(16.))
                .child(div().font_semibold().child(t!("settings.rounding")))
                .child(
                    div()
                        .text_color(p.muted)
                        .child(t!("settings.rounding_detail")),
                ),
        );
        if self.saved.is_none() {
            return match self.status.error.as_ref() {
                Some(error) if !self.status.loading => card.child(
                    v_flex()
                        .gap_3()
                        .px(px(16.))
                        .pb(px(16.))
                        .child(
                            Alert::error("settings-load-error", error.text())
                                .title(t!("settings.load_failed")),
                        )
                        .child(
                            h_flex().child(
                                Button::new("retry")
                                    .text_label(t!("common.retry"))
                                    .outline()
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.refresh(window, cx)),
                                    ),
                            ),
                        ),
                ),
                _ if self.status.loading_visible => {
                    card.children(ROUNDINGS.map(|(_, title, _)| option_skeleton(t!(title), cx)))
                }
                _ => card,
            };
        }
        let disabled = self.status.busy || self.status.loading;
        let changed = self.has_changes();
        let options = RadioGroup::new("dividend-rounding")
            .w_full()
            .disabled(disabled)
            .selected_index(
                ROUNDINGS
                    .iter()
                    .position(|(rounding, ..)| Some(*rounding) == self.picked),
            )
            .on_change(cx.listener(|this, index: &usize, _, cx| this.pick(ROUNDINGS[*index].0, cx)))
            .children(
                ROUNDINGS
                    .iter()
                    .enumerate()
                    .map(|(index, (rounding, title, detail))| {
                        let checked = Some(*rounding) == self.picked;
                        Radio::new(index)
                            .accessibility_label(t!(*title))
                            .w_full()
                            .gap_x(px(12.))
                            .px(px(16.))
                            .py(px(10.))
                            .rounded_none()
                            .border_t_1()
                            .border_color(p.line)
                            .text_size(px(13.))
                            // The group puts a fixed `gap_3` between radios;
                            // the rows touch, as in the mockup.
                            .when(index > 0, |row| row.mt(rems(-0.75)))
                            .when(index == ROUNDINGS.len() - 1 && !changed, |row| {
                                row.rounded_b(data::CARD_INNER_RADIUS)
                            })
                            .when(checked, |row| row.bg(p.selected))
                            .when(!checked && !disabled, |row| {
                                row.hover(|style| style.bg(p.hover))
                            })
                            .child(
                                h_flex()
                                    .gap(px(8.))
                                    .child(div().flex_1().font_medium().child(t!(*title)))
                                    .when(Some(*rounding) == self.saved, |row| {
                                        row.child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.faint)
                                                .child(t!("settings.current")),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(t!(*detail)),
                            )
                    }),
            );
        card.child(
            div()
                .w_full()
                .track_focus(&self.rounding_focus)
                .on_key_down(cx.listener(Self::step_rounding))
                .child(options),
        )
        .when(changed, |card| card.child(self.save_bar(cx)))
    }

    fn save_bar(&self, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let busy = self.status.busy;
        let note = match self.status.error.as_ref() {
            Some(error) => v_flex()
                .gap(px(2.))
                .text_color(p.danger)
                .child(div().font_medium().child(t!("settings.save_failed")))
                .child(div().text_size(px(12.)).child(error.text())),
            None => div().text_color(p.muted).child(t!("settings.unsaved")),
        };
        h_flex()
            .gap(px(8.))
            .py(px(10.))
            .pl(px(16.))
            .pr(px(12.))
            .border_t_1()
            .border_color(p.line)
            .rounded_b(data::CARD_INNER_RADIUS)
            .bg(p.surface)
            .child(note.flex_1().min_w_0())
            .child(
                header::button("discard-rounding", None, t!("settings.discard"))
                    .ghost()
                    .text_color(p.muted)
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| this.discard(cx))),
            )
            .child(
                header::button(
                    "save-rounding",
                    None,
                    if busy {
                        t!("common.saving")
                    } else {
                        t!("settings.save")
                    },
                )
                .primary()
                .disabled(busy)
                .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
            )
    }

    fn appearance_card(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let theme = THEMES.map(|dark| (dark, theme_label(dark)));
        let language = LANGUAGES.map(|language| (language, language_label(language)));
        data::card(cx)
            .child(self.segmented(
                "theme",
                t!("settings.theme"),
                theme,
                theme_choice(cx),
                |this| &this.theme_focus,
                PageContext::set_theme,
                window,
                cx,
            ))
            .child(
                self.segmented(
                    "language",
                    t!("settings.language"),
                    language,
                    language_choice(cx),
                    |this| &this.language_focus,
                    PageContext::set_language,
                    window,
                    cx,
                )
                .border_t_1()
                .border_color(palette(cx).line),
            )
    }

    /// The kit's `ToggleGroup` drops clicks made with Enter or Space, so the
    /// segments are the base `Toggle`, which takes them, each with its own
    /// handler. The base toggle draws nothing, so the page draws the chosen
    /// fill, a hover that only changes the text colour, and the kit's ring
    /// while a segment has keyboard focus.
    #[allow(clippy::too_many_arguments)]
    fn segmented<T: Copy + PartialEq + 'static>(
        &self,
        id: &'static str,
        label: Cow<'static, str>,
        options: [(T, Cow<'static, str>); 3],
        current: T,
        focus: fn(&Self) -> &[FocusHandle; 3],
        pick: fn(&PageContext, T, &mut App),
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = *palette(cx);
        let chosen = cx.theme().tokens.accent;
        h_flex()
            .min_h(px(52.))
            .gap(px(12.))
            .pl(px(16.))
            .pr(px(12.))
            .child(div().flex_1().font_medium().child(label))
            .child(
                h_flex()
                    .gap(px(2.))
                    .p(px(3.))
                    .rounded(px(9.))
                    .bg(cx.theme().button)
                    .border_1()
                    .border_color(p.line)
                    .on_key_down(cx.listener(move |this, event, window, cx| {
                        Self::step_segment(focus(this), event, window, cx)
                    }))
                    .children(options.into_iter().zip(focus(self)).enumerate().map(
                        |(index, ((value, label), focus))| {
                            let checked = value == current;
                            let context = self.context.clone();
                            base::Toggle::new((id, index))
                                .track_focus(focus)
                                .pressed(checked)
                                .accessibility_label(label.clone())
                                .h(px(24.))
                                .px(px(12.))
                                .rounded(px(6.))
                                .text_size(px(12.5))
                                .font_medium()
                                .text_color(if checked { p.text } else { p.muted })
                                .when(checked, |segment| segment.bg(chosen))
                                .when(!checked, |segment| {
                                    segment.hover(|style| style.text_color(p.text))
                                })
                                .when(
                                    window.last_input_was_keyboard() && focus.is_focused(window),
                                    |segment| segment.focus_ring_style(window, cx),
                                )
                                .on_change(move |_, _, _, cx| {
                                    if !checked {
                                        pick(&context, value, cx);
                                    }
                                })
                                .child(label)
                        },
                    )),
            )
    }
}

fn option_skeleton(title: Cow<'static, str>, cx: &App) -> Div {
    h_flex()
        .items_start()
        .gap(px(12.))
        .px(px(16.))
        .py(px(10.))
        .border_t_1()
        .border_color(palette(cx).line)
        .child(
            div()
                .mt(px(2.))
                .size(px(14.))
                .flex_shrink_0()
                .rounded_full()
                .bg(cx.theme().skeleton),
        )
        .child(
            v_flex()
                .gap(px(4.))
                .child(div().font_medium().child(title))
                .child(
                    div()
                        .h(px(12.))
                        .w(px(260.))
                        .rounded(px(4.))
                        .bg(cx.theme().skeleton),
                ),
        )
}
