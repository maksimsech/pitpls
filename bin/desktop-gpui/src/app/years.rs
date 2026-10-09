use super::*;
use crate::{
    components::header,
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    assets::IconName,
    base::actions::Confirm,
    component::{button::*, input::Input},
};
use pitpls_app::use_case::year::{self, YearInfo};

impl Desktop {
    pub fn load_years(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(context) = &self.context else {
            return;
        };
        // Its own task, so a reload doesn't cancel a year being saved.
        self.years_task = Some(context.services.run(
            window,
            cx,
            |app| async move { year::list_year_info(&app).await },
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
        if self.status.busy {
            return;
        }
        match form::year(&self.year_input, cx) {
            Ok(year) => self.change_year(year, false, window, cx),
            Err(error) => {
                self.year_error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn change_year(
        &mut self,
        year: i32,
        remove: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.status.busy {
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
            move |app| async move {
                if remove {
                    year::delete_year(&app, year).await.map(|_| ())
                } else {
                    year::add_year(&app, year).await
                }
            },
            move |this, result, window, cx| {
                this.status.busy = false;
                match result {
                    Ok(()) => {
                        // An added year is selected; a removed one that was
                        // selected falls back to all years.
                        let selected = if remove {
                            this.preferences.year.filter(|selected| *selected != year)
                        } else {
                            this.year_menu_open = false;
                            Some(year)
                        };
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

    /// The tax year popover: all years, then every year newest first with its
    /// record count, an inline field to add one, and a line of help.
    pub(super) fn year_menu(&mut self, cx: &mut Context<Self>) -> Div {
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
                // The kit draws a focus ring 3px outside a row. The padding
                // leaves it room inside the scroll clip, and the negative
                // margin keeps the rows where they were.
                v_flex()
                    .id("year-menu-years")
                    .m(-RING_ROOM)
                    .p(RING_ROOM)
                    .max_h(px(224.) + RING_ROOM * 2.)
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
                                    .h(CONTROL_HEIGHT)
                                    .font_features(tabular_digits())
                                    .aria_label("Add year"),
                            ),
                    )
                    .child(
                        header::button("add-year", None, "Add")
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
                    .child(
                        "Years with records appear on their own. \
                         Removing a year only hides it here.",
                    ),
            )
    }

    /// One row of the year menu; `None` is "All years".
    fn year_item(&self, info: Option<YearInfo>, cx: &mut Context<Self>) -> Button {
        let p = *palette(cx);
        let year = info.map(|info| info.year);
        let label = header::year_label(year);
        let selected = self.preferences.year == year;
        let meta: SharedString = match info.map(|info| info.records()) {
            None => "historical".into(),
            Some(0) => "no records".into(),
            Some(1) => "1 record".into(),
            Some(count) => format!("{count} records").into(),
        };
        // Years with records come back on their own, so only empty custom
        // years can be removed.
        let removable = info.filter(|info| info.custom && info.records() == 0);
        let group = SharedString::from(format!("year-row-{label}"));
        Button::new(SharedString::from(format!("year-{label}")))
            .ghost()
            .w_full()
            .h(px(32.))
            .pl(px(10.))
            .pr(px(6.))
            .rounded(px(7.))
            .group(group.clone())
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
                    )
                    .when_some(removable, |row, info| {
                        let year = info.year;
                        row.child(
                            Button::new(("remove-year", year as u64))
                                .ghost()
                                .size(px(22.))
                                .p_0()
                                .ml(px(4.))
                                .rounded(px(6.))
                                .text_color(p.muted)
                                .child(Icon::new(IconName::X).size(px(13.)))
                                .opacity(0.)
                                .group_hover(group, |style| style.opacity(1.))
                                .tab_stop(false)
                                .accessibility_label(format!("Remove {year}"))
                                .tooltip(format!("Remove {year} from the list"))
                                .disabled(self.status.busy)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.change_year(year, true, window, cx);
                                })),
                        )
                    }),
            )
            // The × shows only on hover, so it's no Tab stop. Backspace or
            // Delete on the row removes the year instead.
            .when_some(removable, |row, info| {
                let year = info.year;
                row.on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let key = &event.keystroke;
                    if matches!(key.key.as_str(), "backspace" | "delete")
                        && !key.modifiers.modified()
                    {
                        cx.stop_propagation();
                        this.change_year(year, true, window, cx);
                    }
                }))
            })
            .on_click(cx.listener(move |this, _, window, cx| this.select_year(year, window, cx)))
    }
}

const CONTROL_HEIGHT: Pixels = px(28.);
/// The width of the kit's focus ring, drawn outside a control.
const RING_ROOM: Pixels = px(3.);
