use super::*;
use chrono::Datelike;
use gpui_kit::component::{button::*, dialog::Dialog};
use pitpls_app::use_case::year;

impl Desktop {
    pub fn load_years(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(context) = &self.context else {
            return;
        };
        self.status.begin_load(window, cx, |this| &mut this.status);
        self.status.task = Some(context.services.run(
            window,
            cx,
            |app| async move { year::list_years(&app).await },
            |this, result, _, cx| {
                if let Some(years) = this.status.loaded(result) {
                    this.years = years;
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn open_year_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.manage_years {
            window.close_dialog(cx);
            self.manage_years = false;
        }
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

    fn close_year_form(&mut self, cx: &mut Context<Self>) {
        self.year_form = None;
        self.status.error = None;
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
                    this.close_year_form(cx);
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
            .w(px(420.))
            .max_w(px(420.))
            .overlay_closable(!self.status.busy)
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
            .on_close(cx.listener(|this, _, _, cx| {
                if !this.status.busy {
                    this.close_year_form(cx);
                }
            }))
            .child(self.status.render())
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
                                    this.close_year_form(cx);
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
        window.open_dialog(cx, move |dialog, _, cx| {
            desktop
                .update(cx, |_, cx| {
                    let confirm = cx.entity().downgrade();
                    dialog
                        .title("Remove year")
                        .footer(components::confirmation_footer("Remove"))
                        .child(format!("Remove {year} from the year selector?"))
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
                .unwrap_or_else(|_| Dialog::new(cx))
        });
        cx.notify();
    }

    pub fn open_year_manager(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.manage_years = true;
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            desktop
                .update(cx, |this, cx| {
                    dialog
                        .title("Manage years")
                        .w(px(420.))
                        .max_w(px(420.))
                        .overlay_closable(!this.status.busy)
                        .keyboard(!this.status.busy)
                        .close_button(!this.status.busy)
                        .on_close(cx.listener(|this, _, _, cx| {
                            this.manage_years = false;
                            cx.notify();
                        }))
                        .child(this.year_manager(cx))
                        .footer(
                            h_flex()
                                .justify_end()
                                .gap_2()
                                .child(
                                    Button::new("done-years")
                                        .label("Done")
                                        .outline()
                                        .disabled(this.status.busy)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            window.close_dialog(cx);
                                            this.manage_years = false;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("add-year")
                                        .label("Add year")
                                        .primary()
                                        .disabled(this.status.busy)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.open_year_form(window, cx)
                                        })),
                                ),
                        )
                })
                .unwrap_or_else(|_| Dialog::new(cx))
        });
        cx.notify();
    }

    fn year_manager(&self, cx: &mut Context<Self>) -> Div {
        let disabled = self.status.busy || self.status.loading || self.delete_year.is_some();
        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Removing a year only removes it from the selector."),
            )
            .child(
                v_flex()
                    .id("year-list")
                    .max_h(px(256.))
                    .overflow_y_scroll()
                    .gap_1()
                    .when(self.years.is_empty(), |view| {
                        view.child("No custom years yet.")
                    })
                    .children(self.years.iter().map(|year| {
                        let year = *year;
                        h_flex()
                            .justify_between()
                            .gap_3()
                            .p_2()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(year.to_string())
                            .child(
                                Button::new(SharedString::from(format!("remove-year-{year}")))
                                    .label("Remove")
                                    .danger()
                                    .small()
                                    .disabled(disabled)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.confirm_remove_year(year, window, cx)
                                    })),
                            )
                    })),
            )
    }
}
