use super::*;
use chrono::Datelike;
use gpui_kit::component::{
    button::*,
    dialog::{AlertDialog, Dialog, DialogButtonProps},
};
use pitpls_app::use_case::year;

impl Desktop {
    pub fn load_years(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        let mut choices = vec![Choice::new(None, "All years")];
        choices.extend(
            years
                .iter()
                .map(|year| Choice::new(Some(*year), year.to_string())),
        );
        let value = self.preferences.year;
        self.year_select.update(cx, |state, cx| {
            state.set_items(choices, window, cx);
            state.set_selected_value(&value, window, cx);
        });
    }

    fn open_year_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.return_focus = window.focused(cx);
        let year = form::input(chrono::Local::now().year().to_string(), window, cx);
        self.year_form = Some(year.clone());
        self.status.error = None;
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            desktop
                .update(cx, |this, cx| this.year_dialog(dialog, cx))
                .unwrap_or_else(|_| Dialog::new(cx))
        });
        window.focus(&year.focus_handle(cx), cx);
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
        let Some(year) = &self.year_form else {
            return;
        };
        let year = form::required(year, "Year", cx).and_then(|value| {
            value
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
                    if this.year_form.is_some() {
                        window.close_dialog(cx);
                    }
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

    fn year_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(year) = &self.year_form else {
            return dialog;
        };
        let dismiss = cx.entity().downgrade();
        let submit = cx.entity().downgrade();
        dialog
            .title("Add custom year")
            .overlay_closable(false)
            .keyboard(!self.status.busy)
            .close_button(!self.status.busy)
            .on_ok(move |_, window, cx| {
                let _ = submit.update(cx, |this, cx| this.add_year(window, cx));
                false
            })
            .on_cancel(move |_, _, cx| {
                dismiss
                    .update(cx, |this, _| !this.status.busy)
                    .unwrap_or(true)
            })
            .on_close(cx.listener(|this, _, window, cx| {
                if !this.status.busy {
                    this.close_year_form(window, cx);
                }
            }))
            .child(self.status.render(cx))
            .child(form::input_field("Year", year, self.status.busy, cx))
            .footer(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-year")
                            .label("Cancel")
                            .outline()
                            .disabled(self.status.busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if !this.status.busy {
                                    window.close_dialog(cx);
                                    this.close_year_form(window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("save-year")
                            .label("Add")
                            .primary()
                            .disabled(self.status.busy)
                            .on_click(cx.listener(|this, _, window, cx| this.add_year(window, cx))),
                    ),
            )
    }

    fn confirm_remove_year(&mut self, year: i32, window: &mut Window, cx: &mut Context<Self>) {
        self.delete_year = Some(year);
        let desktop = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, cx| {
            desktop
                .update(cx, |_, cx| {
                    let confirm = cx.entity().downgrade();
                    dialog
                        .title("Remove year")
                        .child(format!("Remove {year} from the year selector?"))
                        .button_props(
                            DialogButtonProps::default()
                                .show_cancel(true)
                                .ok_text("Remove")
                                .cancel_text("Cancel"),
                        )
                        .on_ok(move |_, window, cx| {
                            confirm
                                .update(cx, |this, cx| this.change_year(year, true, window, cx))
                                .is_ok()
                        })
                        .on_close(cx.listener(|this, _, _, cx| {
                            this.delete_year = None;
                            cx.notify();
                        }))
                })
                .unwrap_or_else(|_| AlertDialog::new(cx))
        });
        cx.notify();
    }

    pub fn year_manager(&self, cx: &mut Context<Self>) -> Div {
        let mut content = v_flex().gap_4().p_5();
        let disabled = self.status.busy
            || self.status.loading
            || self.year_form.is_some()
            || self.delete_year.is_some();
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
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.confirm_remove_year(year, window, cx)
                            }))
                    })),
            );
        content
    }
}
