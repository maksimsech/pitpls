use super::sidebar::CONTROL;
use super::*;
use crate::{
    components::{BUTTON_TEXT, FOCUS_RING, header},
    format::record_count,
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    assets::IconName,
    base::{FocusTrapElement, actions::Confirm},
    component::{button::*, input::Input},
};
use pitpls_app::use_case::year::{self, YearInfo};
use rust_i18n::t;

#[derive(Clone, Copy)]
pub(super) enum Removal {
    Year(i32),
    All,
}

impl Desktop {
    pub fn load_years(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(context) = &self.context else {
            return;
        };
        // Its own task, so a reload doesn't cancel a year being saved.
        self.years_task = Some(context.services.run(
            window,
            cx,
            |app| async move { Ok(year::list_year_info(&app).await?) },
            |this, result, _, cx| {
                match result {
                    Ok(years) => this.years = years,
                    Err(error) => this.status.error = Some(error.into()),
                }
                cx.notify();
            },
        ));
    }

    pub(super) fn set_year_menu(
        &mut self,
        open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if open && self.locked() {
            return;
        }
        self.year_menu_open = open;
        self.year_error = None;
        if open {
            self.year_input
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
    }

    pub(super) fn select_year(
        &mut self,
        year: Option<i32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.locked() {
            return;
        }
        self.year_menu_open = false;
        if self.preferences.year != year {
            self.preferences.year = year;
            self.save_preferences(window, cx);
            self.mount_page(window, cx);
        }
        cx.notify();
    }

    pub(super) fn add_year(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        match form::year(&self.year_input, cx) {
            Ok(year) => self.save_year(year, window, cx),
            Err(error) => {
                self.year_error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn save_year(&mut self, year: i32, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let Some(context) = &self.context else {
            return;
        };
        self.year_error = None;
        self.status.begin_save();
        self.status.task = Some(context.services.run(
            window,
            cx,
            move |app| async move { Ok(year::add_year(&app, year).await?) },
            move |this, result, window, cx| {
                this.status.busy = false;
                match result {
                    Ok(()) => {
                        this.year_menu_open = false;
                        let selected = Some(year);
                        if selected != this.preferences.year {
                            this.preferences.year = selected;
                            this.save_preferences(window, cx);
                            this.mount_page(window, cx);
                        }
                        this.load_years(window, cx);
                    }
                    Err(error) if this.year_menu_open => this.year_error = Some(error.into()),
                    Err(error) => this.status.error = Some(error.into()),
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn confirm_year_removal(
        &mut self,
        removal: Removal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.locked() || self.context.is_none() {
            return;
        }
        self.year_menu_open = false;
        self.year_error = None;
        self.pending_year_removal = Some(removal);
        let (title, message, label) = match removal {
            Removal::Year(year) => (
                t!("year.remove_title", year = year),
                t!("year.remove_message", year = year),
                t!("year.remove", year = year),
            ),
            Removal::All => (
                t!("year.remove_all_title"),
                t!("year.remove_all_message"),
                t!("year.remove_all"),
            ),
        };
        window.focus(&self.focus, cx);
        dialog::confirm(
            title,
            message,
            label,
            Self::remove_years,
            |this, cx| {
                this.pending_year_removal = None;
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }

    fn remove_years(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.page_locked {
            return;
        }
        let (Some(removal), Some(context)) = (self.pending_year_removal, &self.context) else {
            return;
        };
        self.status.begin_save();
        // A pre-deletion year load must not restore stale counts afterward.
        self.years_task = None;
        self.status.task = Some(context.services.run(
            window,
            cx,
            move |app| async move {
                let count = match removal {
                    Removal::Year(year) => year::delete_year(&app, year).await?,
                    Removal::All => year::delete_all_years(&app).await?,
                };
                Ok(crate::i18n::plural("records.deleted", count).into_owned())
            },
            move |this, result, window, cx| {
                if this.status.saved(result) {
                    this.preferences.year = match removal {
                        Removal::Year(year) => this.preferences.year.filter(|value| *value != year),
                        Removal::All => None,
                    };
                    // The previous import summary no longer describes the stored records.
                    this.preferences.last_import = None;
                    this.save_preferences(window, cx);
                    // Deleting another year can change holdings and FIFO costs on this page too.
                    this.mount_page(window, cx);
                    window.focus(&this.focus, cx);
                }
                this.load_years(window, cx);
                cx.notify();
            },
        ));
        cx.notify();
    }

    pub(super) fn year_menu(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = *palette(cx);
        let busy = self.status.busy;
        let mut years = self.years.clone();
        // The saved year may have neither records nor an entry.
        if let Some(selected) = self.preferences.year
            && !years.iter().any(|info| info.year == selected)
        {
            years.push(YearInfo {
                year: selected,
                dividends: 0,
                interests: 0,
                cryptos: 0,
                stocks: 0,
                custom: false,
            });
            years.sort_unstable_by_key(|info| std::cmp::Reverse(info.year));
        }
        let divider = || div().h(px(1.)).mx(px(2.)).my(px(4.)).bg(p.line);
        v_flex()
            .w_full()
            .text_size(px(13.))
            .child(self.year_item(None, cx))
            .child(divider())
            .child(
                div()
                    .pt(px(2.))
                    .px(px(10.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .font_medium()
                    .text_color(p.faint)
                    .child(t!("year.tax_years")),
            )
            .child(
                // The padding leaves a row's focus ring room inside the
                // scroll clip, and the negative margin keeps the rows where
                // they were.
                v_flex()
                    .id("year-menu-years")
                    .m(-FOCUS_RING)
                    .p(FOCUS_RING)
                    .max_h(px(224.) + FOCUS_RING * 2.)
                    .overflow_y_scroll()
                    .children(years.into_iter().map(|info| self.year_item(Some(info), cx))),
            )
            .child(divider())
            .child(
                h_flex()
                    .gap(px(6.))
                    .pt(px(4.))
                    .px(px(4.))
                    .pb(px(2.))
                    .child(
                        // A single-line input passes Enter on, and the
                        // popover binds Enter and Space to toggling itself,
                        // which would close the menu instead of adding.
                        div()
                            .flex_1()
                            .min_w_0()
                            .on_action(|_: &Confirm, _, _| {})
                            .child(
                                // Not disabled while saving, so it keeps focus
                                // for a correction; Enter waits until then.
                                Input::new(&self.year_input)
                                    .h(CONTROL)
                                    .font_features(tabular_digits())
                                    .aria_label(t!("year.add_label")),
                            ),
                    )
                    .child(
                        header::button("add-year", None, t!("year.add"))
                            .loading(busy)
                            .on_click(cx.listener(|this, _, window, cx| this.add_year(window, cx))),
                    ),
            )
            .when_some(self.year_error.clone(), |menu, error| {
                menu.child(
                    div()
                        .px(px(8.))
                        .pt(px(6.))
                        .text_size(px(12.))
                        .text_color(p.danger)
                        .child(error),
                )
            })
            .child(
                div()
                    .px(px(8.))
                    .pt(px(6.))
                    .pb(px(4.))
                    .text_size(px(11.5))
                    .line_height(relative(1.35))
                    .text_color(p.faint)
                    .child(t!("year.menu_hint")),
            )
            .child(divider())
            .child(
                header::button(
                    "remove-all-years",
                    Some(IconName::Trash),
                    t!("year.remove_all"),
                )
                .ghost()
                .w_full()
                .text_color(p.danger)
                .disabled(self.locked())
                .on_click(cx.listener(|this, _, window, cx| {
                    this.confirm_year_removal(Removal::All, window, cx);
                })),
            )
            .focus_trap("year-menu-trap", &self.year_menu_focus)
    }

    fn year_item(&self, info: Option<YearInfo>, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let year = info.map(|info| info.year);
        let label = header::year_label(year);
        let selected = self.preferences.year == year;
        let meta: SharedString = match info {
            Some(info) => record_count(info.records()).into(),
            None => t!("year.all_meta").into(),
        };
        let select = Button::new(SharedString::from(format!("year-{label}")))
            .ghost()
            .flex_1()
            .min_w_0()
            .h(px(32.))
            .pl(px(10.))
            .pr(px(6.))
            .rounded(px(7.))
            .role(Role::MenuItemRadio)
            .accessibility_label(format!("{label}, {meta}"))
            .disabled(self.locked())
            .child(
                h_flex()
                    .w_full()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .w(px(14.))
                            .when(selected, |check| {
                                check.child(Icon::new(IconName::Check).size(px(14.)))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_left()
                            .text_size(BUTTON_TEXT)
                            .when(selected, |label| label.font_medium())
                            .child(label.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .font_features(tabular_digits())
                            .child(meta),
                    ),
            )
            .when_some(year, |row, year| {
                row.on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let key = &event.keystroke;
                    if matches!(key.key.as_str(), "backspace" | "delete")
                        && !key.modifiers.modified()
                    {
                        cx.stop_propagation();
                        this.confirm_year_removal(Removal::Year(year), window, cx);
                    }
                }))
            })
            .on_click(cx.listener(move |this, _, window, cx| this.select_year(year, window, cx)));
        h_flex()
            .w_full()
            .gap(px(2.))
            .child(select)
            .when_some(year, |row, year| {
                row.child(
                    Button::new(("remove-year", year as u64))
                        .ghost()
                        .size(px(24.))
                        .flex_shrink_0()
                        .p_0()
                        .rounded(px(6.))
                        .text_color(p.muted)
                        .hover(|style| style.text_color(p.danger))
                        .child(Icon::new(IconName::X).size(px(14.)))
                        .accessibility_label(t!("year.remove_tooltip", year = year))
                        .tooltip(t!("year.remove_tooltip", year = year))
                        .disabled(self.locked())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.confirm_year_removal(Removal::Year(year), window, cx);
                        })),
                )
            })
    }
}
