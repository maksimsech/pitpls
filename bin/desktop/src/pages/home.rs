use crate::{
    components::{self, ButtonText, Status, data, header, nbp, notice, value},
    config::LastImport,
    format::{DisplayText, pln, record_count},
    i18n::{and_list, or_list, plural, plural_key},
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
    stock::Listings,
    tax::{self, Error as TaxError, TaxSummaryData},
    year::{self, YearInfo},
};
use pitpls_core::{stock::Warning, summary::StockTaxSummary};
use pitpls_importers::{IMPORTERS, InputType};
use rust_decimal::Decimal;
use rust_i18n::t;

/// The labels are message keys.
struct Forms {
    shares: [(&'static str, DisplayText); 2],
    pit_zg: Option<PitZg>,
    foreign: [(&'static str, DisplayText); 3],
    /// A plain subtraction of the two totals, never rounded.
    difference: DisplayText,
    crypto: [(&'static str, DisplayText); 2],
}

struct PitZg {
    countries: Vec<PitZgCountry>,
    no_exchange: Option<SharedString>,
}

struct PitZgCountry {
    code: SharedString,
    detail: SharedString,
    // None for a country without income: no PIT/ZG, or 0 in field 29. The
    // labels are message keys.
    fields: Option<[(&'static str, DisplayText); 2]>,
    profit: DisplayText,
    loss: bool,
}

impl Forms {
    fn new(data: &TaxSummaryData) -> Self {
        let summary = &data.summary;
        let foreign = &summary.foreign;
        let stocks = &summary.stocks;
        Self {
            shares: [
                ("tax.income_c22", pln(stocks.income)),
                ("tax.costs_c23", pln(stocks.costs)),
            ],
            pit_zg: PitZg::new(stocks, &data.listings),
            foreign: [
                ("tax.income_i65", pln(foreign.income)),
                ("tax.to_pay_g47", pln(foreign.tax_to_pay)),
                ("tax.paid_g48", pln(foreign.tax_paid)),
            ],
            difference: pln(foreign.tax_to_pay - foreign.tax_paid),
            crypto: [
                ("tax.income_e36", pln(summary.crypto.income)),
                ("tax.costs_e37", pln(summary.crypto.costs)),
            ],
        }
    }
}

impl PitZg {
    // Also shown when every sale lacks an exchange, so the line pointing to the warning isn't lost.
    fn new(stocks: &StockTaxSummary, listings: &Listings) -> Option<Self> {
        let mut missing: Vec<&str> = Vec::new();
        for warning in &stocks.warnings {
            if let Warning::MissingExchange { ticker, .. } = warning
                && !missing.contains(&ticker.as_str())
            {
                missing.push(ticker);
            }
        }
        if stocks.countries.is_empty() && missing.is_empty() {
            return None;
        }
        let no_exchange = (!missing.is_empty()).then(|| {
            let key = plural_key("home.no_exchange", missing.len() as u64);
            t!(key, tickers = and_list(&missing)).into()
        });

        let countries = stocks
            .countries
            .iter()
            .map(|country| {
                let exchanges = country
                    .exchanges
                    .iter()
                    .map(|mic| {
                        listings
                            .name(*mic)
                            .map_or_else(|| mic.to_string(), str::to_owned)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                PitZgCountry {
                    code: country.country.to_string().into(),
                    detail: format!(
                        "{exchanges} · {}",
                        plural("stocks.sales_count", country.sales as u64)
                    )
                    .into(),
                    fields: (country.income > Decimal::ZERO).then(|| {
                        [
                            ("tax.income_zg29", pln(country.income)),
                            ("tax.paid_abroad_zg30", pln(country.tax_paid)),
                        ]
                    }),
                    profit: pln(country.profit),
                    loss: country.profit < Decimal::ZERO,
                }
            })
            .collect();

        Some(Self {
            countries,
            no_exchange,
        })
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
    stocks: u32,
    first_run: bool,
    coverage: Option<RateCoverage>,
    // None on All years: tax totals only mean something per tax year.
    totals: Option<Totals>,
}

impl Overview {
    fn new(
        year: Option<i32>,
        years: &[YearInfo],
        coverage: Option<RateCoverage>,
        summary: Option<Result<TaxSummaryData, TaxError>>,
    ) -> Self {
        let selected = years
            .iter()
            .filter(|info| year.is_none_or(|year| info.year == year));
        let (dividends, interests, cryptos, stocks) = selected.fold(
            (0, 0, 0, 0),
            |(dividends, interests, cryptos, stocks), info| {
                (
                    dividends + info.dividends,
                    interests + info.interests,
                    cryptos + info.cryptos,
                    stocks + info.stocks,
                )
            },
        );
        let totals = summary.map(|summary| match summary {
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
        });
        Self {
            dividends,
            interests,
            cryptos,
            stocks,
            first_run: coverage.is_none() && years.iter().all(|info| info.records() == 0),
            coverage,
            totals,
        }
    }

    fn records(&self) -> u32 {
        self.dividends + self.interests + self.cryptos + self.stocks
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
                let summary = match year {
                    Some(year) => Some(tax::load_tax_summary(&app, year).await),
                    None => None,
                };
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
                    header::button("summary-import", IconName::Upload, t!("common.import"))
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
                |column| column.child(self.status.render(|this| &mut this.status, cx)),
            );
        let column = match &self.overview {
            Some(overview) => column
                .children(
                    overview
                        .totals
                        .as_ref()
                        .map(|totals| self.totals(totals, overview, cx)),
                )
                .child(self.data_card(overview, cx))
                .child(notice::disclaimer(cx)),
            None if self.status.loading_visible => column.when(self.year.is_some(), |column| {
                column.children(self.skeleton(cx))
            }),
            None if !self.status.loading => column.child(
                h_flex().child(
                    Button::new("retry")
                        .text_label(t!("common.retry"))
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))),
                ),
            ),
            None => column,
        };
        div().w_full().px(px(20.)).child(column)
    }

    fn totals(&self, totals: &Totals, overview: &Overview, cx: &mut Context<Self>) -> AnyElement {
        let disabled = self.locked();
        match totals {
            Totals::Ready(forms) => {
                let foreign_page = overview.foreign_page();
                v_flex()
                    .gap(px(16.))
                    .child(self.shares_card(forms, cx))
                    .children(
                        forms
                            .pit_zg
                            .as_ref()
                            .map(|pit_zg| self.pit_zg_card(pit_zg, cx)),
                    )
                    .child(
                        card(
                            t!("home.foreign"),
                            None,
                            Some(
                                link("foreign-records", t!("home.records"), disabled, cx).on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.context.navigate(foreign_page, cx)
                                    }),
                                ),
                            ),
                            cx,
                        )
                        .children(forms.foreign.iter().map(|(label, value)| {
                            value::card_row(form_id(label), t!(*label), value, false, cx)
                        }))
                        .child(value::card_row(
                            "form-difference",
                            t!("tax.difference_g47_g48"),
                            &forms.difference,
                            true,
                            cx,
                        )),
                    )
                    .child(
                        card(
                            t!("nav.crypto"),
                            None,
                            Some(
                                link("crypto-records", t!("home.records"), disabled, cx).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.context.navigate(Page::Crypto, cx)
                                    }),
                                ),
                            ),
                            cx,
                        )
                        .children(forms.crypto.iter().map(
                            |(label, value)| {
                                value::card_row(form_id(label), t!(*label), value, false, cx)
                            },
                        )),
                    )
                    .into_any_element()
            }
            Totals::MissingRate(error) => notice::missing_rate(
                self.year,
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
                        .title(t!("home.calculation_failed")),
                )
                .child(
                    h_flex().child(
                        Button::new("retry")
                            .text_label(t!("common.retry"))
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
        let rates = data::rate_status("home.latest_rate", overview.coverage, self.year, cx);
        let update = data::is_current_year(self.year).then(|| {
            header::button(
                "update-rates",
                None,
                if self.updating {
                    t!("home.updating")
                } else {
                    t!("home.update")
                },
            )
            .when(rates.stale, |button| button.primary())
            .disabled(self.disabled())
            .on_click(cx.listener(|this, _, window, cx| this.update_rates(window, cx)))
        });
        let rates = data_row(
            rates.dot,
            t!("home.exchange_rates"),
            rates.detail,
            update,
            cx,
        );

        let import = data::import_status(self.last_import.as_ref(), cx);
        let last_import = data_row(
            import.dot,
            t!("home.last_import"),
            data::import_detail(&import),
            import.date.map(|date| {
                div()
                    .text_color(p.faint)
                    .font_features(tabular_digits())
                    .child(date)
            }),
            cx,
        );

        card(
            t!("nav.data"),
            None,
            Some(
                link("data-imports", t!("nav.imports"), self.locked(), cx).on_click(
                    cx.listener(|this, _, _, cx| this.context.navigate(Page::Imports, cx)),
                ),
            ),
            cx,
        )
        .child(rates)
        .child(last_import)
    }

    fn shares_card(&self, forms: &Forms, cx: &mut Context<Self>) -> Div {
        card(
            t!("home.shares"),
            None,
            Some(
                link("stock-records", t!("home.records"), self.locked(), cx).on_click(
                    cx.listener(|this, _, _, cx| this.context.navigate(Page::Stocks, cx)),
                ),
            ),
            cx,
        )
        .children(
            forms.shares.iter().map(|(label, value)| {
                value::card_row(form_id(label), t!(*label), value, false, cx)
            }),
        )
    }

    fn pit_zg_card(&self, pit_zg: &PitZg, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let (income, losses): (Vec<_>, Vec<_>) = pit_zg
            .countries
            .iter()
            .partition(|country| country.fields.is_some());
        card("PIT/ZG", Some(t!("home.pit_zg_caption").into()), None, cx)
            .children(income.into_iter().flat_map(|country| {
                let rows = country.fields.iter().flatten().map(|(label, value)| {
                    value::card_row(
                        SharedString::from(format!("pit-zg-{}-{label}", country.code)),
                        t!(*label),
                        value,
                        false,
                        cx,
                    )
                });
                std::iter::once(
                    h_flex()
                        .h(px(40.))
                        .px(px(16.))
                        .gap(px(8.))
                        .border_t_1()
                        .border_color(p.line)
                        .bg(p.hover)
                        .child(div().font_semibold().child(country.code.clone()))
                        .child(div().flex_1())
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(country.detail.clone()),
                        ),
                )
                .chain(rows)
                .collect::<Vec<_>>()
            }))
            .children(losses.into_iter().map(|country| {
                let result = if country.loss {
                    t!("home.loss")
                } else {
                    t!("home.profit")
                };
                h_flex()
                    .min_h(px(44.))
                    .px(px(16.))
                    .py(px(8.))
                    .gap(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .text_color(p.muted)
                    .child(
                        div()
                            .font_medium()
                            .text_color(p.text)
                            .child(country.code.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(country.detail.clone()),
                    )
                    .child(div().flex_1())
                    .child(
                        sentence(
                            SharedString::from(format!("pit-zg-{}-profit", country.code)),
                            &[
                                Part::Text(result.into_owned()),
                                Part::Value(&country.profit),
                                Part::Text(t!("home.no_pit_zg").into_owned()),
                            ],
                            cx,
                        )
                        .justify_end(),
                    )
            }))
            .when_some(pit_zg.no_exchange.clone(), |card, text| {
                card.child(
                    h_flex()
                        .min_h(px(44.))
                        .pl(px(16.))
                        .pr(px(10.))
                        .py(px(8.))
                        .gap(px(8.))
                        .border_t_1()
                        .border_color(p.line)
                        .child(
                            Icon::new(IconName::MapPin)
                                .size(px(14.))
                                .flex_shrink_0()
                                .text_color(p.warning),
                        )
                        .child(div().flex_1().min_w_0().text_color(p.muted).child(text))
                        .child(
                            link("pit-zg-stocks", t!("nav.stocks"), self.locked(), cx).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.context.navigate(Page::Stocks, cx)
                                }),
                            ),
                        ),
                )
            })
    }

    fn skeleton(&self, cx: &App) -> [Div; 3] {
        let rows = |labels: &[&'static str]| {
            labels
                .iter()
                .map(|label| value::card_row_skeleton(t!(*label), true, cx))
                .collect::<Vec<_>>()
        };
        [
            card(t!("home.shares"), None, None, cx)
                .children(rows(&["tax.income_c22", "tax.costs_c23"])),
            card(t!("home.foreign"), None, None, cx).children(rows(&[
                "tax.income_i65",
                "tax.to_pay_g47",
                "tax.paid_g48",
                "tax.difference_g47_g48",
            ])),
            card(t!("nav.crypto"), None, None, cx)
                .children(rows(&["tax.income_e36", "tax.costs_e37"])),
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
        let statements = or_list(&statements);
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
                                .child(t!("home.welcome")),
                        )
                        .child(div().text_color(p.muted).child(t!("home.welcome_detail"))),
                )
                .child(
                    data::card(cx)
                        .child(step(
                            1,
                            t!("home.step_rates"),
                            t!("home.step_rates_detail"),
                            true,
                            Some(
                                header::button("first-run-rates", None, t!("nbp.import_title"))
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
                            t!("home.step_statement"),
                            t!("home.step_statement_detail", statements = statements),
                            false,
                            Some(
                                header::button("first-run-imports", None, t!("home.open_imports"))
                                    .disabled(disabled)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.context.navigate(Page::Imports, cx)
                                    })),
                            ),
                            cx,
                        ))
                        .child(step(
                            3,
                            t!("home.step_copy"),
                            t!("home.step_copy_detail"),
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

fn card(
    title: impl Into<SharedString>,
    caption: Option<SharedString>,
    link: Option<Button>,
    cx: &App,
) -> Div {
    let p = palette(cx);
    data::card(cx).child(
        h_flex()
            .h(px(44.))
            .pl(px(16.))
            .pr(px(10.))
            .gap(px(8.))
            .child(div().font_semibold().child(title.into()))
            .children(caption.map(|caption| {
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(caption)
            }))
            .child(div().flex_1())
            .children(link),
    )
}

enum Part<'a> {
    Text(String),
    Value(&'a DisplayText),
}

// Each word is its own element so the line wraps like text around values that keep their
// full-value tooltip. Text that doesn't start with a space sticks to the value before it.
fn sentence(id: impl Into<ElementId>, parts: &[Part], cx: &App) -> Stateful<Div> {
    let mut words: Vec<Div> = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        match part {
            Part::Text(text) => {
                let mut glued = !text.starts_with(char::is_whitespace);
                for word in text.split_whitespace() {
                    let word = word.to_owned();
                    match words.pop() {
                        Some(last) if glued => words.push(last.child(word)),
                        last => {
                            words.extend(last);
                            words.push(h_flex().child(word));
                        }
                    }
                    glued = false;
                }
            }
            Part::Value(value) => {
                words.push(h_flex().child(value::reveal_full(
                    div().id(index).child(value::text(value, false, false, cx)),
                    value,
                )));
                words.extend(value.unit.clone().map(|unit| h_flex().child(unit)));
            }
        }
    }
    h_flex()
        .id(id)
        .flex_wrap()
        .gap_x(px(3.5))
        .font_features(tabular_digits())
        .children(words)
}

fn link(id: &'static str, label: impl Into<SharedString>, disabled: bool, cx: &App) -> Button {
    let label = label.into();
    Button::new(id)
        .ghost()
        .text_color(palette(cx).muted)
        .h(px(28.))
        .px(px(6.))
        .rounded(px(7.))
        .accessibility_label(label.clone())
        .disabled(disabled)
        .child(div().text_size(px(12.)).child(label))
        .child(Icon::new(IconName::ChevronRight).size(px(13.)))
}

fn data_row(
    dot: Hsla,
    label: impl Into<SharedString>,
    detail: impl IntoElement,
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
        .child(
            div()
                .w(px(120.))
                .flex_shrink_0()
                .font_medium()
                .child(label.into()),
        )
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
    title: impl Into<SharedString>,
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
                .child(div().font_medium().child(title.into()))
                .child(div().text_color(p.muted).child(description.into())),
        )
        .children(button.map(|button| div().flex_shrink_0().child(button)))
}
