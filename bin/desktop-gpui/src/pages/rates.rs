use super::PageView;
use crate::{
    components::{
        self, Status, file_picker,
        form::Form,
        table::{Column, cell},
    },
    navigation::PageContext,
};
use chrono::Datelike;
use gpui_kit::{
    component::{button::*, scroll::ScrollableElement, *},
    *,
};
use pitpls_app::use_case::rate;
use serde_json::json;
use std::ops::Range;

#[derive(Default)]
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
    form: Option<Form>,
    confirm_reset: bool,
    return_focus: Option<FocusHandle>,
    rate_scroll: UniformListScrollHandle,
}

impl RatesPage {
    pub fn new(context: PageContext, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            context,
            status: Status::default(),
            data: RateData::default(),
            form: None,
            confirm_reset: false,
            return_focus: None,
            rate_scroll: UniformListScrollHandle::new(),
        };
        page.refresh(window, cx);
        page
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(
            self.status.busy || self.form.is_some() || self.confirm_reset,
            cx,
        );
        cx.notify();
    }

    fn close_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.form = None;
        self.status.error = None;
        if let Some(focus) = self.return_focus.take() {
            window.focus(&focus, cx);
        }
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
                    this.close_form(window, cx);
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
        self.return_focus = window.focused(cx);
        let mut form = Form::new("Import rates from NBP");
        form.input(
            "year",
            "Year",
            &json!({"year": chrono::Local::now().year().to_string()}),
            false,
            window,
            cx,
        );
        form.focus(window, cx);
        self.form = Some(form);
        self.status.error = None;
        self.notify(cx);
    }

    fn import_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &self.form else {
            return;
        };
        let current_year = chrono::Local::now().year();
        let year = form
            .values(cx)
            .and_then(|value| {
                value["year"]
                    .as_str()
                    .unwrap_or_default()
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

    fn controls(&self, cx: &mut Context<Self>) -> Div {
        let disabled = self.status.busy || self.status.loading || self.confirm_reset;
        let mut content = v_flex().gap_4().p_5().child(self.status.render(cx));
        if let Some(form) = &self.form {
            return content
                .child(div().text_lg().font_semibold().child(form.title.clone()))
                .child(form.render(self.status.busy, cx))
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("cancel-nbp")
                                .label("Cancel")
                                .outline()
                                .disabled(self.status.busy)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.close_form(window, cx)),
                                ),
                        )
                        .child(
                            Button::new("import-nbp")
                                .label("Import")
                                .primary()
                                .disabled(self.status.busy)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.import_nbp(window, cx)),
                                ),
                        ),
                );
        }
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
                        .label("Reset rates")
                        .outline()
                        .disabled(disabled || !self.status.ready || self.data.rows.is_empty())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.confirm_reset = true;
                            this.notify(cx);
                        })),
                ),
        );
        if self.confirm_reset {
            content = content
                .child(components::notice(
                    "Remove all imported exchange rates? Calculations will be unavailable until rates are reimported.",
                    cx,
                ))
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("cancel-reset")
                                .label("Cancel")
                                .outline()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.confirm_reset = false;
                                    this.notify(cx);
                                })),
                        )
                        .child(
                            Button::new("confirm-reset")
                                .label("Confirm reset")
                                .primary()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.change(Change::Reset, window, cx)
                                })),
                        ),
                );
        }
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
        if self.status.busy || self.form.is_some() || self.confirm_reset {
            return;
        }
        self.status.begin_load();
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
                        let mut cells = vec![row.date.into()];
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
                if let Some(data) = this.status.loaded(result) {
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
        if self.form.is_some() || !self.status.ready {
            return components::scroll(self.controls(cx));
        }
        v_flex()
            .flex_1()
            .min_h_0()
            .child(self.controls(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .px_5()
                    .pb_5()
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
        // Rate rows are uniform, unlike the expandable financial record rows.
        // Keep their header and virtual list in one horizontal scroll container.
        let table = v_flex()
            .w(rems(width / 14.))
            .h_full()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex().h(px(42.)).bg(cx.theme().muted).children(
                    self.data
                        .columns
                        .iter()
                        .map(|column| cell(column.label.clone(), column, true, cx)),
                ),
            )
            .child(
                uniform_list(
                    "rate-days",
                    self.data.rows.len(),
                    cx.processor(|this, range: Range<usize>, _, cx| {
                        range
                            .map(|index| {
                                h_flex()
                                    .h(px(40.))
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .children(
                                        this.data.rows[index].iter().zip(&this.data.columns).map(
                                            |(value, column)| {
                                                cell(value.clone(), column, false, cx)
                                            },
                                        ),
                                    )
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1()
                .min_h_0()
                .track_scroll(&self.rate_scroll),
            );
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .id("rates-horizontal")
                    .size_full()
                    .overflow_x_scrollbar()
                    .child(table),
            )
            .vertical_scrollbar(&self.rate_scroll)
            .into_any_element()
    }
}
