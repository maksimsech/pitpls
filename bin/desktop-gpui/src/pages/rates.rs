use super::PageView;
use crate::{
    components::{
        self, Status, dialog, file_picker, header, nbp,
        table::{Column, cell},
    },
    format,
    navigation::{Page, PageContext},
};
use chrono::{Datelike, NaiveDate};
use gpui_kit::component::{dialog::Dialog, input::InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
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
        let year = nbp::year_input(chrono::Local::now().year(), window, cx);
        self.nbp_year = Some(year.clone());
        self.status.error = None;
        dialog::open(window, cx, Self::nbp_dialog);
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
        match nbp::year(year, cx) {
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
        nbp::dialog(
            self,
            dialog,
            year,
            |this| &this.status,
            Self::import_nbp,
            Self::close_form,
            cx,
        )
    }

    fn open_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_reset = true;
        dialog::confirm(
            "Reset rates",
            "Remove all imported exchange rates? \
             Calculations will be unavailable until rates are reimported.",
            "Reset",
            |this, window, cx| this.change(Change::Reset, window, cx),
            |this, cx| {
                this.confirm_reset = false;
                this.notify(cx);
            },
            window,
            cx,
        );
        self.notify(cx);
    }

    fn disabled(&self) -> bool {
        self.status.busy || self.status.loading || self.nbp_year.is_some() || self.confirm_reset
    }

    fn header(&self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let disabled = self.disabled();
        // Rows run from the oldest day to the newest.
        let range = match (self.data.rows.first(), self.data.rows.last()) {
            (Some(first), Some(last)) => format!("NBP table A · {} – {}", first[0], last[0]),
            _ => "NBP table A".into(),
        };
        header::page(Page::Rates.title(), Some(range.into()), window, cx).child(
            header::actions()
                .child(self.status.refreshing(cx))
                .child(
                    header::button("rates-csv", IconName::Upload, "Upload CSV")
                        .disabled(disabled)
                        .on_click(cx.listener(|this, _, window, cx| this.pick_file(window, cx))),
                )
                .child(
                    header::button("rates-nbp", IconName::Download, "Import from NBP")
                        .primary()
                        .disabled(disabled)
                        .on_click(cx.listener(|this, _, window, cx| this.open_nbp(window, cx))),
                )
                .child(
                    header::button("rates-reset", None, "Reset")
                        .ghost()
                        .text_color(cx.theme().danger)
                        .disabled(disabled || !self.status.ready || self.data.rows.is_empty())
                        .on_click(cx.listener(|this, _, window, cx| this.open_reset(window, cx))),
                ),
        )
    }

    /// Operation results and, after a failed load, Retry.
    fn notices(&self, cx: &mut Context<Self>) -> Div {
        let disabled = self.disabled();
        let mut content = v_flex()
            .gap_4()
            .flex_shrink_0()
            .when(self.status.is_visible(), |view| {
                view.child(self.status.render())
            });
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
        if self.disabled() {
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
                            .map(|day| format::date(day).main)
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header = self.header(window, cx);
        let page = v_flex().size_full().min_h_0().child(header);
        if !self.status.ready {
            return page.child(components::scroll(
                components::page_content()
                    .gap_4()
                    .child(self.notices(cx))
                    .when(self.status.loading, |view| {
                        view.child(components::records::skeleton(
                            &[Column::text("Date", 130.), Column::number("Rate", 125.)],
                            self.status.loading_visible,
                            cx,
                        ))
                    }),
            ));
        }
        page.child(
            components::page_content()
                .relative()
                .min_w_0()
                .overflow_hidden()
                .flex_1()
                .min_h_0()
                .gap_4()
                .child(self.notices(cx))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .overflow_hidden()
                        .child(self.rates(cx)),
                ),
        )
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
                                    .map(|(value, column)| cell(value.clone(), column, false, cx)),
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
