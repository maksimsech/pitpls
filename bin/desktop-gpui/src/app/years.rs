use super::*;
use chrono::Datelike;
use gpui_kit::component::button::*;
use pitpls_app::use_case::year;
use serde_json::json;

impl Desktop {
    pub(super) fn load_years(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(context) = &self.context else {
            return;
        };
        self.status.begin_load();
        self.status.task = Some(context.services.run(
            window,
            cx,
            |app| async move { year::list_years(&app).await },
            |this, result, window, cx| {
                if let Some(years) = this.status.loaded(result) {
                    this.years = years;
                    this.sync_year_select(window, cx);
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn sync_year_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut years = self.years.clone();
        if let Some(year) = self.preferences.year
            && !years.contains(&year)
        {
            years.push(year);
        }
        years.sort_unstable_by(|a, b| b.cmp(a));
        let mut choices = vec![Choice::new("all", "All years")];
        choices.extend(
            years
                .iter()
                .map(|year| Choice::new(year.to_string(), year.to_string())),
        );
        let value: SharedString = self
            .preferences
            .year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "all".into())
            .into();
        self.year_select.update(cx, |state, cx| {
            state.set_items(choices, window, cx);
            state.set_selected_value(&value, window, cx);
        });
    }

    fn open_year_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.return_focus = window.focused(cx);
        let mut form = Form::new("Add custom year");
        form.input(
            "year",
            "Year",
            &json!({"year": chrono::Local::now().year().to_string()}),
            false,
            window,
            cx,
        );
        form.focus(window, cx);
        self.year_form = Some(form);
        self.status.error = None;
        cx.notify();
    }

    fn close_year_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.year_form = None;
        self.status.error = None;
        if let Some(focus) = self.return_focus.take() {
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    fn add_year(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &self.year_form else {
            return;
        };
        let year = form.values(cx).and_then(|value| {
            value["year"]
                .as_str()
                .unwrap_or_default()
                .parse::<i32>()
                .map_err(|_| "Enter a valid whole year".to_string())
        });
        match year {
            Ok(year) => self.change_year(year, false, window, cx),
            Err(error) => {
                self.status.error = Some(error.into());
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
        if self.status.busy || self.status.loading {
            return;
        }
        let Some(context) = &self.context else {
            return;
        };
        self.delete_year = None;
        self.status.begin_save();
        self.status.task = Some(context.services.run(
            window,
            cx,
            move |app| async move {
                if remove {
                    year::delete_year(&app, year).await?;
                    Ok(format!(
                        "Removed {year} from the year selector. Financial records are unchanged."
                    ))
                } else {
                    year::add_year(&app, year).await?;
                    Ok(format!("Added {year} to the year selector."))
                }
            },
            move |this, result, window, cx| {
                if this.status.saved(result) {
                    this.close_year_form(window, cx);
                    if remove {
                        if this.preferences.year == Some(year) {
                            this.preferences.year = None;
                        }
                    } else {
                        this.preferences.year = Some(year);
                    }
                    this.save_preferences(window, cx);
                    this.mount_page(window, cx);
                    this.load_years(window, cx);
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    pub(super) fn year_manager(&self, cx: &mut Context<Self>) -> Div {
        let mut content = v_flex().gap_4().p_5();
        if let Some(form) = &self.year_form {
            return content
                .child(div().text_lg().font_semibold().child(form.title.clone()))
                .child(form.render(self.status.busy, cx))
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("cancel-year")
                                .label("Cancel")
                                .outline()
                                .disabled(self.status.busy)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_year_form(window, cx)
                                })),
                        )
                        .child(
                            Button::new("save-year")
                                .label("Save")
                                .primary()
                                .disabled(self.status.busy)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.add_year(window, cx)),
                                ),
                        ),
                );
        }
        let disabled = self.status.busy || self.status.loading || self.delete_year.is_some();
        content = content
            .child(
                h_flex()
                    .justify_between()
                    .child(div().font_semibold().child("Manage years"))
                    .child(
                        Button::new("add-year")
                            .label("Add year")
                            .outline()
                            .disabled(disabled)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.open_year_form(window, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Removing a year only removes it from the selector."),
            )
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_2()
                    .children(self.years.iter().map(|year| {
                        let year = *year;
                        Button::new(SharedString::from(format!("remove-year-{year}")))
                            .label(format!("Remove {year}"))
                            .outline()
                            .disabled(disabled)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_year = Some(year);
                                cx.notify();
                            }))
                    })),
            );
        if let Some(year) = self.delete_year {
            content = content
                .child(components::notice(
                    format!("Remove {year} from the year selector?"),
                    cx,
                ))
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("cancel-remove-year")
                                .label("Cancel")
                                .outline()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.delete_year = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("confirm-remove-year")
                                .label("Confirm")
                                .primary()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.change_year(year, true, window, cx)
                                })),
                        ),
                );
        }
        content
    }
}
