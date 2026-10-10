use crate::{
    components::{self, ButtonText, Status, data, header, nbp, notice, value},
    config::LastImport,
    format::{DisplayText, pln, record_count},
    navigation::{Page, PageContext},
    services::Error as ServiceError,
    theme::{palette, tabular_digits},
};
use chrono::{Datelike, Local};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{alert::Alert, button::*, dialog::Dialog, input::InputState, *},
    *,
};
use pitpls_app::use_case::{
    rate::{self, RateCoverage},
    tax::{self, Error as TaxError},
    year::{self, YearInfo},
};
use pitpls_core::summary::TaxSummary;
use pitpls_importers::{IMPORTERS, InputType};

struct Forms {
    foreign: [(&'static str, DisplayText); 3],
    /// A plain subtraction of the two totals, never rounded.
    difference: DisplayText,
    crypto: [(&'static str, DisplayText); 2],
}

impl Forms {
    fn new(summary: &TaxSummary) -> Self {
        let foreign = &summary.foreign;
        Self {
            foreign: [
                ("Income (I-65)", pln(foreign.income)),
                ("Tax to pay (G-47)", pln(foreign.tax_to_pay)),
                ("Paid tax (G-48)", pln(foreign.tax_paid)),
            ],
            difference: pln(foreign.tax_to_pay - foreign.tax_paid),
            crypto: [
                ("Income (E-36)", pln(summary.crypto.income)),
                ("Costs (E-37)", pln(summary.crypto.costs)),
            ],
        }
    }
}

enum Totals {
    Ready(Box<Forms>),
    MissingRate(SharedString),
    Failed(SharedString),
}

struct Overview {
    dividends: u32,
    interests: u32,
    cryptos: u32,
    first_run: bool,
    coverage: Option<RateCoverage>,
    totals: Totals,
}

impl Overview {
    fn new(
        year: Option<i32>,
        years: &[YearInfo],
        coverage: Option<RateCoverage>,
        summary: Result<TaxSummary, TaxError>,
    ) -> Self {
        let selected = years
            .iter()
            .filter(|info| year.is_none_or(|year| info.year == year));
        let (dividends, interests, cryptos) =
            selected.fold((0, 0, 0), |(dividends, interests, cryptos), info| {
                (
                    dividends + info.dividends,
                    interests + info.interests,
                    cryptos + info.cryptos,
                )
            });
        let totals = match summary {
            Ok(summary) => Totals::Ready(Box::new(Forms::new(&summary))),
            Err(error) => {
                let error = ServiceError::from(error);
                let missing_rate = error.missing_rate();
                let message = error.into();
                if missing_rate {
                    Totals::MissingRate(message)
                } else {
                    Totals::Failed(message)
                }
            }
        };
        Self {
            dividends,
            interests,
            cryptos,
            first_run: coverage.is_none() && years.iter().all(|info| info.records() == 0),
            coverage,
            totals,
        }
    }

    fn records(&self) -> u32 {
        self.dividends + self.interests + self.cryptos
    }

    fn foreign_page(&self) -> Page {
        if self.dividends == 0 && self.interests > 0 {
            Page::Interests
        } else {
            Page::Dividends
        }
    }
}

pub struct HomePage {
    context: PageContext,
    year: Option<i32>,
    last_import: Option<LastImport>,
    status: Status,
    overview: Option<Overview>,
    nbp_year: Option<Entity<InputState>>,
    updating: bool,
    focus: FocusHandle,
}

impl HomePage {
    pub fn new(
        context: PageContext,
        year: Option<i32>,
        last_import: Option<LastImport>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut page = Self {
            context,
            year,
            last_import,
            status: Status::default(),
            overview: None,
            nbp_year: None,
            updating: false,
            focus: cx.focus_handle(),
        };
        page.refresh(window, cx);
        page
    }

    fn locked(&self) -> bool {
        self.status.busy || self.nbp_year.is_some()
    }

    fn disabled(&self) -> bool {
        self.locked() || self.status.loading
    }

    fn notify(&self, cx: &mut Context<Self>) {
        self.context.set_locked(self.locked(), cx);
        cx.notify();
    }

    fn update_rates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        self.updating = true;
        self.import_rates(Local::now().year(), window, cx);
    }

    fn import_rates(&mut self, year: i32, window: &mut Window, cx: &mut Context<Self>) {
        self.status.begin_save();
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| nbp::import(app, year),
            |this, result, window, cx| {
                this.updating = false;
                if this.status.saved(result) {
                    if this.nbp_year.is_some() {
                        window.close_dialog(cx);
                        this.close_nbp(cx);
                        window.focus(&this.focus, cx);
                    }
                    this.refresh(window, cx);
                }
                this.notify(cx);
            },
        ));
        self.notify(cx);
    }

    fn open_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled() {
            return;
        }
        let year = self.year.unwrap_or_else(|| Local::now().year());
        self.nbp_year = Some(nbp::open(year, Self::nbp_dialog, window, cx));
        self.status.error = None;
        self.status.message = None;
        self.notify(cx);
    }

    fn close_nbp(&mut self, cx: &mut Context<Self>) {
        self.nbp_year = None;
        self.status.error = None;
        self.notify(cx);
    }

    fn import_nbp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        let Some(input) = &self.nbp_year else {
            return;
        };
        match nbp::year(input, cx) {
            Ok(year) => self.import_rates(year, window, cx),
            Err(error) => {
                self.status.error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn nbp_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let Some(input) = &self.nbp_year else {
            return dialog;
        };
        nbp::dialog(
            self,
            dialog,
            input,
            |this| &this.status,
            Self::import_nbp,
            Self::close_nbp,
            cx,
        )
    }

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.loading || self.locked() {
            return;
        }
        self.status.begin_load(window, cx, |this| &mut this.status);
        let year = self.year;
        self.status.task = Some(self.context.services.run(
            window,
            cx,
            move |app| async move {
                let years = year::list_year_info(&app).await?;
                let coverage = rate::rate_coverage(&app).await?;
                let summary = tax::load_tax_summary(&app, year).await;
                Ok(Overview::new(year, &years, coverage, summary))
            },
            |this, result, _, cx| {
                if let Some(overview) = this.status.loaded(result) {
                    this.overview = Some(overview);
                }
                this.context.loaded(cx);
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl Render for HomePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let first_run = self
            .overview
            .as_ref()
            .is_some_and(|overview| overview.first_run);
        let content = if first_run {
            self.welcome(cx)
        } else {
            self.summary(cx)
        };
        v_flex()
            .id("summary-page")
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .text_size(px(13.))
            .child(self.header(first_run, window, cx))
            .child(components::scroll(content))
    }
}

impl HomePage {
    fn header(
        &self,
        first_run: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        if first_run {
            return header::page(Page::Home.title(), None, window, cx);
        }
        let year = header::year_label(self.year);
        let label = match &self.overview {
            Some(overview) => format!("{year} · {}", record_count(overview.records())).into(),
            None => year,
        };
        let disabled = self.disabled();
        header::page(Page::Home.title(), Some(label), window, cx).child(
            header::actions()
                .child(self.status.refreshing(cx))
                .child(header::refresh(disabled, cx).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.context.years_changed(cx);
                        this.refresh(window, cx);
                    },
                )))
                .child(
                    header::button("summary-import", IconName::Upload, "Import")
                        .disabled(self.locked())
                        .on_click(
                            cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx)),
                        ),
                ),
        )
    }

    fn summary(&self, cx: &mut Context<Self>) -> Div {
        let column = v_flex()
            .w_full()
            .max_w(px(680.))
            .mx_auto()
            .pt(px(14.))
            .pb(px(32.))
            .gap(px(16.))
            // The NBP dialog shows its own errors.
            .when(
                self.status.is_visible() && self.nbp_year.is_none(),
                |column| column.child(self.status.render()),
            );
        let column = match &self.overview {
            Some(overview) => column
                .child(self.totals(overview, cx))
                .child(self.data_card(overview, cx))
                .child(notice::disclaimer(cx)),
            None if self.status.loading_visible => column.children(self.skeleton(cx)),
            None if !self.status.loading => column.child(
                h_flex().child(
                    Button::new("retry")
                        .text_label("Retry")
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            ),
            None => column,
        };
        div().w_full().px(px(20.)).child(column)
    }

    fn totals(&self, overview: &Overview, cx: &mut Context<Self>) -> AnyElement {
        let disabled = self.locked();
        match &overview.totals {
            Totals::Ready(forms) => {
                let foreign_page = overview.foreign_page();
                v_flex()
                    .gap(px(16.))
                    .child(
                        card(
                            "Foreign dividends and interest",
                            Some(link("foreign-records", "Records", disabled, cx).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.context.navigate(foreign_page, cx)
                                }),
                            )),
                            cx,
                        )
                        .children(forms.foreign.iter().map(|(label, value)| {
                            value::card_row(form_id(label), *label, value, false, cx)
                        }))
                        .child(value::card_row(
                            "form-difference",
                            "Difference (G-47 − G-48)",
                            &forms.difference,
                            true,
                            cx,
                        )),
                    )
                    .child(
                        card(
                            "Crypto",
                            Some(link("crypto-records", "Records", disabled, cx).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.context.navigate(Page::Crypto, cx)
                                }),
                            )),
                            cx,
                        )
                        .children(forms.crypto.iter().map(
                            |(label, value)| {
                                value::card_row(form_id(label), *label, value, false, cx)
                            },
                        )),
                    )
                    .into_any_element()
            }
            Totals::MissingRate(error) => notice::missing_rate(
                self.year,
                error.clone(),
                self.disabled(),
                cx.listener(|this, _, window, cx| this.open_nbp(window, cx)),
                cx.listener(|this, _, _, cx| this.context.navigate(Page::Rates, cx)),
                cx,
            )
            .pb(px(16.))
            .into_any_element(),
            Totals::Failed(error) => v_flex()
                .gap_3()
                .child(
                    Alert::error("summary-error", error.clone())
                        .title("Couldn't calculate the summary"),
                )
                .child(
                    h_flex().child(
                        Button::new("retry")
                            .text_label("Retry")
                            .outline()
                            .disabled(self.disabled())
                            .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                    ),
                )
                .into_any_element(),
        }
    }

    fn data_card(&self, overview: &Overview, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let rates = data::rate_status("Latest NBP rate", overview.coverage, self.year, cx);
        let update = data::is_current_year(self.year).then(|| {
            header::button(
                "update-rates",
                None,
                if self.updating {
                    "Updating…"
                } else {
                    "Update"
                },
            )
            .when(rates.stale, |button| button.primary())
            .disabled(self.disabled())
            .on_click(cx.listener(|this, _, window, cx| this.update_rates(window, cx)))
        });
        let rates = data_row(rates.dot, "Exchange rates", rates.detail, update, cx);

        let import = data::import_status(self.last_import.as_ref(), cx);
        let last_import = data_row(
            import.dot,
            "Last import",
            StyledText::new(import.detail),
            import.date.map(|date| {
                div()
                    .text_color(p.faint)
                    .font_features(tabular_digits())
                    .child(date)
            }),
            cx,
        );

        card(
            "Data",
            Some(
                link("data-imports", "Imports", self.locked(), cx).on_click(
                    cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx)),
                ),
            ),
            cx,
        )
        .child(rates)
        .child(last_import)
    }

    fn skeleton(&self, cx: &App) -> [Div; 2] {
        let rows = |labels: &[&'static str]| {
            labels
                .iter()
                .map(|label| value::card_row_skeleton(label, true, cx))
                .collect::<Vec<_>>()
        };
        [
            card("Foreign dividends and interest", None, cx).children(rows(&[
                "Income (I-65)",
                "Tax to pay (G-47)",
                "Paid tax (G-48)",
                "Difference (G-47 − G-48)",
            ])),
            card("Crypto", None, cx).children(rows(&["Income (E-36)", "Costs (E-37)"])),
        ]
    }

    fn welcome(&self, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let disabled = self.disabled();
        let statements = IMPORTERS
            .iter()
            .map(|importer| {
                let formats = importer
                    .input
                    .iter()
                    .map(|input| match input {
                        InputType::Csv => "CSV",
                        InputType::Pdf => "PDF",
                    })
                    .collect::<Vec<_>>()
                    .join("/");
                format!(
                    "{} {formats}",
                    crate::messages::importer_name(importer.kind)
                )
            })
            .collect::<Vec<_>>();
        let statements = match statements.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
            None => String::new(),
        };
        div().w_full().px(px(20.)).pt(px(56.)).pb(px(32.)).child(
            v_flex()
                .w_full()
                .max_w(px(560.))
                .mx_auto()
                .gap(px(18.))
                .child(
                    v_flex()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(20.))
                                .font_semibold()
                                .child("Welcome to pitpls"),
                        )
                        .child(
                            div()
                                .text_color(p.muted)
                                .child("Three steps to the numbers for your PIT-38."),
                        ),
                )
                .child(
                    data::card(cx)
                        .child(step(
                            1,
                            "Import exchange rates",
                            "Every amount is converted with the NBP table A rate from the \
                             previous working day.",
                            true,
                            Some(
                                header::button("first-run-rates", None, "Import from NBP")
                                    .primary()
                                    .disabled(disabled)
                                    .on_click(
                                        cx.listener(|this, _, window, cx| {
                                            this.open_nbp(window, cx)
                                        }),
                                    ),
                            ),
                            cx,
                        ))
                        .child(step(
                            2,
                            "Import a statement",
                            format!("{statements}. You can also add records by hand."),
                            false,
                            Some(
                                header::button("first-run-imports", None, "Open imports")
                                    .disabled(disabled)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.context.navigate(Page::Imports, cx)
                                    })),
                            ),
                            cx,
                        ))
                        .child(step(
                            3,
                            "Copy the totals",
                            "Summary lists each form field with a copy button, ready for the \
                             tax office site.",
                            false,
                            None,
                            cx,
                        )),
                )
                .child(notice::disclaimer(cx)),
        )
    }
}

fn form_id(label: &'static str) -> SharedString {
    format!("form-{label}").into()
}

fn card(title: &'static str, link: Option<Button>, cx: &App) -> Div {
    data::card(cx).child(
        h_flex()
            .h(px(44.))
            .pl(px(16.))
            .pr(px(10.))
            .justify_between()
            .child(div().font_semibold().child(title))
            .children(link),
    )
}

fn link(id: &'static str, label: &'static str, disabled: bool, cx: &App) -> Button {
    Button::new(id)
        .ghost()
        .text_color(palette(cx).muted)
        .h(px(28.))
        .px(px(6.))
        .rounded(px(7.))
        .accessibility_label(label)
        .disabled(disabled)
        .child(div().text_size(px(12.)).child(label))
        .child(Icon::new(IconName::ChevronRight).size(px(13.)))
}

fn data_row(
    dot: Hsla,
    label: &'static str,
    detail: StyledText,
    end: Option<impl IntoElement>,
    cx: &App,
) -> Div {
    let p = palette(cx);
    h_flex()
        .h(px(48.))
        .pl(px(16.))
        .pr(px(12.))
        .gap(px(10.))
        .border_t_1()
        .border_color(p.line)
        .child(div().w(px(16.)).flex_shrink_0().child(data::dot(dot)))
        .child(div().w(px(120.)).flex_shrink_0().font_medium().child(label))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(p.muted)
                .child(detail),
        )
        .children(end.map(|end| h_flex().flex_shrink_0().child(end)))
}

fn step(
    number: u8,
    title: &'static str,
    description: impl Into<SharedString>,
    current: bool,
    button: Option<Button>,
    cx: &App,
) -> Div {
    let p = palette(cx);
    h_flex()
        .items_start()
        .gap(px(14.))
        .p(px(16.))
        .when(number > 1, |step| step.border_t_1().border_color(p.line))
        .child(
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .size(px(24.))
                .mt(px(1.))
                .rounded_full()
                .text_size(px(12.))
                .font_semibold()
                .when(current, |number| {
                    number.bg(p.primary).text_color(p.primary_text)
                })
                .when(!current, |number| {
                    number
                        .border_1()
                        .border_color(p.strong_line)
                        .text_color(p.muted)
                })
                .child(number.to_string()),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(2.))
                .child(div().font_medium().child(title))
                .child(div().text_color(p.muted).child(description.into())),
        )
        .children(button.map(|button| div().flex_shrink_0().child(button)))
}
