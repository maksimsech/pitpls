use super::PageView;
use crate::{
    components::{
        self, Status, file_picker, form,
        table::{Column, cell, value_cell},
    },
    format::{self, DisplayText},
    navigation::PageContext,
};
use chrono::{Datelike, NaiveDate};
use gpui_kit::component::{dialog::Dialog, input::InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{button::*, scroll::ScrollableElement, *},
    *,
};
use pitpls_app::use_case::rate;
use std::ops::Range;

#[derive(Default, PartialEq)]
struct RateData {
    columns: Vec<Column>,
    rows: Vec<Vec<SharedString>>,
}

enum Change {
    Csv(String),
    Nbp(i32),
    Reset,
}

pub struct RatesPage {
    context: PageContext,
    status: Status,
    data: RateData,
    nbp_year: Option<Entity<InputState>>,
    confirm_reset: bool,
    rate_scroll: UniformListScrollHandle,
    horizontal_scroll: ScrollHandle,
}

impl RatesPage {
    pub fn new(context: PageContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            context,
            status: Status::default(),
            data: RateData::default(),
            nbp_year: None,
            confirm_reset: false,
            rate_scroll: UniformListScrollHandle::new(),
            horizontal_scroll: ScrollHandle::new(),
        };
        page.refresh(window, cx);
        page
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(
            self.status.busy || self.nbp_year.is_some() || self.confirm_reset,
            cx,
        );
        cx.notify();
    }

    fn close_form(&mut self, cx: &mut Context<Self>) {
        self.nbp_year = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn change(&mut self, change: Change, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading {
            return;
        }
        self.confirm_reset = false;
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                match change {
                    Change::Csv(file) => Ok(format!(
                        "Imported {} rates.",
                        rate::import_csv(&app, file).await?
                    )),
                    Change::Nbp(year) => Ok(format!(
                        "Imported {} rates.",
                        rate::import_api(&app, year).await?
                    )),
                    Change::Reset => {
                        Ok(format!("Removed {} rates.", rate::reset_rates(&app).await?))
                    }
                }
            },
            |this, result, window, cx| {
                if this.status.saved(result) {
                    if this.nbp_year.is_some() {
                        window.close_dialog(cx);
                    }
                    this.close_form(cx);
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading {
            return;
        }
        self.status.begin_save();
        self.status.task = Some(file_picker::pick(
            "csv",
            window,
            cx,
            |this, result, window, cx| {
                this.status.busy = false;
                match result {
                    Ok(Some(file)) => this.change(Change::Csv(file), window, cx),
                    Ok(None) => {}
                    Err(error) => this.status.error = Some(error.into()),
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn open_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let year = form::input(chrono::Local::now().year().to_string(), window, cx);
        self.nbp_year = Some(year.clone());
        self.status.error = None;
        let page = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            page.update(cx, |this, cx| this.nbp_dialog(dialog, cx))
                .unwrap_or_else(|_| Dialog::new(cx))
        });
        window.focus(&year.focus_handle(cx), cx);
        self.notify(cx);
    }

    fn import_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(year) = &self.nbp_year else {
            return;
        };
        let current_year = chrono::Local::now().year();
        let year = form::required(year, "Year", cx)
            .and_then(|value| {
                value
                    .parse::<i32>()
                    .map_err(|_| "Enter a valid whole year".to_string())
            })
            .and_then(|year| {
                if (2002..=current_year).contains(&year) {
                    Ok(year)
                } else {
                    Err(format!("Enter a year between 2002 and {current_year}"))
                }
            });
        match year {
            Ok(year) => self.change(Change::Nbp(year), window, cx),
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn nbp_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(year) = &self.nbp_year else {
            return dialog;
        };
        let dismiss = cx.entity().downgrade();
        let submit = cx.entity().downgrade();
        dialog
            .title("Import from NBP")
            .overlay_closable(!self.status.busy)
            .keyboard(!self.status.busy)
            .close_button(!self.status.busy)
            .on_ok(move |_, window, cx| {
                let _ = submit.update(cx, |this, cx| this.import_nbp(window, cx));
                false
            })
            .on_cancel(move |_, _, cx| {
                dismiss
                    .update(cx, |this, _| !this.status.busy)
                    .unwrap_or(true)
            })
            .on_close(cx.listener(|this, _, _, cx| {
                if !this.status.busy {
                    this.close_form(cx);
                }
            }))
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render())
            })
            .child(form::input_field("Year", year, self.status.busy, cx))
            .footer(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-nbp")
                            .label("Cancel")
                            .outline()
                            .disabled(self.status.busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if !this.status.busy {
                                    window.close_dialog(cx);
                                    this.close_form(cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("import-nbp")
                            .label(if self.status.busy {
                                "Importing…"
                            } else {
                                "Import"
                            })
                            .primary()
                            .disabled(self.status.busy)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.import_nbp(window, cx)),
                            ),
                    ),
            )
    }

    fn open_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_reset = true;
        let page = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            page.update(cx, |_, cx| {
                let confirm = cx.entity().downgrade();
                dialog
                    .title("Reset rates")
                    .footer(components::confirmation_footer("Reset"))
                    .child(
                        "Remove all imported exchange rates? \
                         Calculations will be unavailable until rates are reimported.",
                    )
                    .on_ok(move |_, window, cx| {
                        confirm
                            .update(cx, |this, cx| this.change(Change::Reset, window, cx))
                            .is_ok()
                    })
                    .on_close(cx.listener(|this, _, _, cx| {
                        this.confirm_reset = false;
                        this.notify(cx);
                    }))
            })
            .unwrap_or_else(|_| Dialog::new(cx))
        });
        self.notify(cx);
    }

    fn controls(&self, cx: &mut Context<Self>) -> Div {
        let disabled = self.status.busy
            || self.status.loading
            || self.nbp_year.is_some()
            || self.confirm_reset;
        let mut content = v_flex()
            .gap_4()
            .flex_shrink_0()
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render())
            });
        content = content.child(
            h_flex()
                .flex_wrap()
                .gap_2()
                .child(
                    Button::new("rates-csv")
                        .label("Upload CSV")
                        .primary()
                        .disabled(disabled)
                        .on_click(cx.listener(|this, _, window, cx| this.pick_file(window, cx))),
                )
                .child(
                    Button::new("rates-nbp")
                        .label("Import from NBP")
                        .outline()
                        .disabled(disabled)
                        .on_click(cx.listener(|this, _, window, cx| this.open_nbp(window, cx))),
                )
                .child(
                    Button::new("rates-reset")
                        .label("Reset")
                        .danger()
                        .disabled(disabled || !self.status.ready || self.data.rows.is_empty())
                        .on_click(cx.listener(|this, _, window, cx| this.open_reset(window, cx))),
                ),
        );
        if self.status.error.is_some() || (!self.status.ready && !self.status.loading) {
            content = content.child(
                Button::new("retry-rates")
                    .label("Retry")
                    .outline()
                    .disabled(disabled)
                    .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
            );
        }
        content
    }
}

impl PageView for RatesPage {
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy || self.status.loading || self.nbp_year.is_some() || self.confirm_reset
        {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            |app| async move {
                let data = rate::list_rates(&app).await?;
                let mut columns = vec![Column::text("Date", 130.)];
                columns.extend(
                    data.currencies
                        .iter()
                        .map(|currency| Column::number(currency.to_string(), 125.)),
                );
                let rows = data
                    .rows
                    .into_iter()
                    .map(|row| {
                        let day = NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
                            .map(|day| format::date(day).text)
                            .unwrap_or_else(|_| row.date.into());
                        let mut cells = vec![day];
                        cells.extend(data.currencies.iter().map(|currency| {
                            row.rates
                                .iter()
                                .find(|rate| rate.currency == *currency)
                                .map(|rate| rate.rate.clone().into())
                                .unwrap_or_else(|| "—".into())
                        }));
                        cells
                    })
                    .collect();
                Ok(RateData { columns, rows })
            },
            |this, result, _, cx| {
                if let Some(data) = this.status.loaded(result)
                    && data != this.data
                {
                    this.data = data;
                    this.rate_scroll = UniformListScrollHandle::new();
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for RatesPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.status.ready {
            return components::scroll(
                components::page_content()
                    .gap_4()
                    .child(self.controls(cx))
                    .when(self.status.loading, |view| {
                        view.child(components::records::skeleton(
                            &[Column::text("Date", 130.), Column::number("Rate", 125.)],
                            self.status.loading_visible,
                            cx,
                        ))
                    }),
            );
        }
        components::page_content()
            .relative()
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .flex_1()
            .min_h_0()
            .gap_4()
            .child(self.controls(cx))
            .child(self.status.refreshing(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(self.rates(cx)),
            )
            .into_any_element()
    }
}

impl RatesPage {
    fn rates(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.data.rows.is_empty() {
            return components::empty("No rates yet. Upload a CSV or import from NBP.", cx)
                .into_any_element();
        }
        let width = self
            .data
            .columns
            .iter()
            .map(|column| column.width)
            .sum::<f32>();
        let mut list = uniform_list(
            "rate-days",
            self.data.rows.len(),
            cx.processor(|this, range: Range<usize>, _, cx| {
                range
                    .map(|index| {
                        h_flex()
                            .id(("rate-day", index))
                            .h(px(40.))
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .children(
                                this.data.rows[index]
                                    .iter()
                                    .zip(&this.data.columns)
                                    .enumerate()
                                    .map(|(column_index, (value, column))| {
                                        value_cell(
                                            ("rate", column_index),
                                            &DisplayText::plain(value.clone()),
                                            column,
                                            cx,
                                        )
                                    }),
                            )
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .flex_1()
        .min_h_0()
        .track_scroll(&self.rate_scroll);
        // Keep horizontal wheel events available to the enclosing viewport.
        list.style().restrict_scroll_to_axis = Some(true);
        let table = v_flex()
            .w(rems(width / 14.))
            .flex_shrink_0()
            .h_full()
            .min_h_0()
            .child(
                h_flex()
                    .flex_shrink_0()
                    .h(px(42.))
                    .rounded_t(cx.theme().radius)
                    .bg(cx.theme().muted)
                    .children(
                        self.data
                            .columns
                            .iter()
                            .map(|column| cell(column.label.clone(), column, true, cx)),
                    ),
            )
            .child(list);
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .child(
                div()
                    .id("rates-horizontal")
                    .size_full()
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.horizontal_scroll)
                    .child(table),
            )
            .child(
                div().absolute().inset_0().child(
                    scroll::Scrollbar::horizontal(&self.horizontal_scroll)
                        .mode(scroll::ScrollbarMode::Always)
                        .viewport_from_layout(),
                ),
            )
            .vertical_scrollbar(&self.rate_scroll)
            .into_any_element()
    }
}
