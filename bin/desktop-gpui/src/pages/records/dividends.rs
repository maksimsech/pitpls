use super::{RecordForm, RecordKind, Submission, conversion, day, percent};
use crate::navigation::Page;
use crate::{
    components::{
        form::{self, ChoiceState},
        records::{CellStyle, Preview, RecordColumn, RowDisplay, Step, StepLine},
    },
    format::{DisplayText, amount, date, money, pln},
};
use chrono::NaiveDate;
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::{
    dividend::{self, CreateDividendInput, UpdateDividendInput},
    year::YearInfo,
};
use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::{CalculatedDividend, Dividend},
    tax::{POLAND_TAX, get_treaty_tax},
};
use pitpls_importers::OutputType;
use rust_decimal::Decimal;
use std::sync::Arc;

pub struct Dividends;

impl RecordKind for Dividends {
    type Record = CalculatedDividend;
    type Form = DividendForm;

    const PAGE: Page = Page::Dividends;
    const NAME: &'static str = "Dividend";
    const PLURAL: &'static str = "dividends";
    const TOTAL_LABELS: &'static [&'static str] =
        &["Income (I-65)", "To pay (G-47)", "Paid (G-48)"];

    fn id(record: &CalculatedDividend) -> &str {
        &record.id
    }

    fn date(record: &CalculatedDividend) -> NaiveDate {
        record.date
    }

    fn columns() -> Vec<RecordColumn> {
        vec![
            RecordColumn::new("Date", CellStyle::Muted, 64., 44.),
            RecordColumn::new("Ticker", CellStyle::Strong, 76., 56.),
            RecordColumn::grow("Provider", CellStyle::Muted),
            RecordColumn::new("Country", CellStyle::Muted, 64., 46.),
            RecordColumn::new("Value", CellStyle::Number, 150., 124.),
            RecordColumn::new("Tax paid", CellStyle::Number, 124., 104.),
        ]
    }

    fn display(record: &CalculatedDividend) -> RowDisplay {
        let cells = vec![
            day(record.date),
            DisplayText::plain(record.ticker.clone()),
            DisplayText::plain(record.provider.clone()),
            DisplayText::plain(record.country.to_string()),
            amount(record.value),
            amount(record.tax_paid),
        ];
        let steps = vec![
            Step {
                title: "Conversion",
                lines: vec![
                    StepLine::Formula(conversion(record.value, record.nbp_rate)),
                    StepLine::Result(vec![pln(record.calculated_value)]),
                    StepLine::Caption(
                        format!("Calculated value · NBP date {}", date(record.nbp_date).main)
                            .into(),
                    ),
                ],
            },
            Step {
                title: "Polish tax",
                lines: vec![
                    StepLine::Formula(vec![
                        DisplayText::plain(format!("{} ×", percent(POLAND_TAX))),
                        money(record.calculated_value),
                    ]),
                    StepLine::Result(vec![pln(record.calculated_to_pay)]),
                    StepLine::Caption("Calculated to pay".into()),
                ],
            },
            Step {
                title: "Foreign tax credit",
                lines: vec![
                    StepLine::Formula(
                        [
                            vec![DisplayText::plain("Tax paid")],
                            conversion(record.tax_paid, record.tax_paid_nbp_rate),
                        ]
                        .concat(),
                    ),
                    StepLine::Entry {
                        label: "Calculated tax paid",
                        value: pln(record.calculated_tax_paid),
                        strong: false,
                    },
                    StepLine::Entry {
                        label: "Max tax paid",
                        value: pln(record.max_tax_paid),
                        strong: false,
                    },
                    StepLine::Entry {
                        label: "Used tax paid",
                        value: pln(record.used_tax_paid),
                        strong: true,
                    },
                    StepLine::Caption(
                        format!(
                            "Max = {} {} treaty rate; the lower one is used",
                            percent(get_treaty_tax(&record.country)),
                            record.country
                        )
                        .into(),
                    ),
                ],
            },
        ];
        RowDisplay {
            cells,
            steps,
            search: format!("{}\n{}\n{}", record.ticker, record.provider, record.country)
                .to_lowercase(),
            label: format!("{} {}", record.ticker, date(record.date).main).into(),
        }
    }

    fn subtotal(records: &[&CalculatedDividend]) -> Vec<(Option<&'static str>, Decimal)> {
        let value = records.iter().map(|record| record.calculated_value).sum();
        vec![(None, value)]
    }

    fn imported(output: &OutputType) -> bool {
        matches!(output, OutputType::Dividend)
    }

    fn count(year: &YearInfo) -> u32 {
        year.dividends
    }

    async fn load(
        app: Arc<pitpls_app::App>,
        year: Option<i32>,
    ) -> Result<(Vec<Decimal>, Vec<CalculatedDividend>), String> {
        let data = dividend::load_dividends(&app, year).await?;
        Ok((vec![data.income, data.to_pay, data.paid], data.calculated))
    }

    async fn save(app: Arc<pitpls_app::App>, submission: DividendSubmission) -> Result<(), String> {
        match submission {
            Submission::Create(input) => dividend::create_dividend(&app, input).await.map(drop),
            Submission::Update(input) => dividend::update_dividend(&app, input).await,
        }
    }

    async fn delete(app: Arc<pitpls_app::App>, ids: Vec<String>) -> Result<u64, String> {
        dividend::delete_dividends(&app, ids).await
    }

    async fn preview(
        app: Arc<pitpls_app::App>,
        draft: DividendDraft,
    ) -> Result<CalculatedDividend, String> {
        let (value, value_currency) = draft.value;
        let (tax_paid, tax_paid_currency) = draft.tax_paid;
        let dividend = Dividend {
            id: String::new(),
            date: draft.date,
            ticker: String::new(),
            value: Amount {
                value,
                currency: value_currency,
            },
            tax_paid: Amount {
                value: tax_paid,
                currency: tax_paid_currency,
            },
            country: draft.country,
            provider: String::new(),
        };
        dividend::preview_dividend(&app, dividend).await
    }

    fn preview_display(record: &CalculatedDividend) -> Preview {
        Preview {
            nbp_date: record.nbp_date,
            formula: conversion(record.value, record.nbp_rate),
            value: pln(record.calculated_value),
            results: vec![
                ("Calculated to pay", pln(record.calculated_to_pay)),
                ("Used tax paid", pln(record.used_tax_paid)),
            ],
        }
    }
}

/// What the dividend calculation reads; ticker and provider don't count.
#[derive(Clone, PartialEq)]
pub struct DividendDraft {
    date: NaiveDate,
    value: (Decimal, Currency),
    tax_paid: (Decimal, Currency),
    country: Country,
}

type DividendSubmission = Submission<CreateDividendInput, UpdateDividendInput>;

pub struct DividendForm {
    existing_id: Option<String>,
    id: Entity<InputState>,
    date: Entity<DatePickerState>,
    ticker: Entity<InputState>,
    value: Entity<InputState>,
    value_currency: Entity<ChoiceState<Currency>>,
    tax_paid: Entity<InputState>,
    tax_paid_currency: Entity<ChoiceState<Currency>>,
    country: Entity<InputState>,
    provider: Entity<InputState>,
}

impl RecordForm for DividendForm {
    type Record = CalculatedDividend;
    type Submission = DividendSubmission;
    type Draft = DividendDraft;

    fn new(record: Option<&CalculatedDividend>, window: &mut Window, cx: &mut App) -> Self {
        let text = |value: fn(&CalculatedDividend) -> String| record.map(value).unwrap_or_default();
        Self {
            existing_id: record.map(|record| record.id.clone()),
            id: form::optional_id(text(|record| record.id.clone()), window, cx),
            date: form::date_picker(
                record.map_or_else(|| chrono::Local::now().date_naive(), |record| record.date),
                window,
                cx,
            ),
            ticker: form::input(text(|record| record.ticker.clone()), window, cx),
            value: form::input(text(|record| record.value.value.to_string()), window, cx),
            value_currency: form::currency(
                record.map_or(Currency::USD, |record| record.value.currency),
                window,
                cx,
            ),
            tax_paid: form::input(
                record.map_or_else(|| "0".into(), |record| record.tax_paid.value.to_string()),
                window,
                cx,
            ),
            tax_paid_currency: form::currency(
                record.map_or(Currency::USD, |record| record.tax_paid.currency),
                window,
                cx,
            ),
            country: form::input(
                record.map_or_else(|| "US".into(), |record| record.country.to_string()),
                window,
                cx,
            ),
            provider: form::input(text(|record| record.provider.clone()), window, cx),
        }
    }

    fn title(&self) -> &'static str {
        if self.existing_id.is_some() {
            "Edit dividend"
        } else {
            "Add dividend"
        }
    }

    fn existing_id(&self) -> Option<&str> {
        self.existing_id.as_deref()
    }

    fn id_input(&self) -> &Entity<InputState> {
        &self.id
    }

    fn first_input(&self) -> &Entity<InputState> {
        &self.ticker
    }

    fn submission(&self, cx: &App) -> Result<DividendSubmission, String> {
        let date = form::selected_date(&self.date, "Date", cx)?;
        let ticker = form::required(&self.ticker, "Ticker", cx)?;
        let value = form::amount(&self.value, "Value", cx)?;
        let value_currency = form::selected(&self.value_currency, "Value currency", cx)?;
        let tax_paid = form::amount(&self.tax_paid, "Tax paid", cx)?;
        let tax_paid_currency = form::selected(&self.tax_paid_currency, "Tax paid currency", cx)?;
        let country = form::required(&self.country, "Country code", cx)?.parse()?;
        let provider = form::required(&self.provider, "Provider", cx)?;
        Ok(match self.existing_id.clone() {
            Some(id) => Submission::Update(UpdateDividendInput {
                id,
                date,
                ticker,
                value,
                value_currency,
                tax_paid,
                tax_paid_currency,
                country,
                provider,
            }),
            None => Submission::Create(CreateDividendInput {
                id: form::optional(&self.id, cx),
                date,
                ticker,
                value,
                value_currency,
                tax_paid,
                tax_paid_currency,
                country,
                provider,
            }),
        })
    }

    fn draft(&self, cx: &App) -> Option<DividendDraft> {
        Some(DividendDraft {
            date: form::picked_date(&self.date, cx)?,
            value: form::parsed_amount(&self.value, &self.value_currency, cx)?,
            tax_paid: form::parsed_amount(&self.tax_paid, &self.tax_paid_currency, cx)?,
            country: form::text(&self.country, cx).parse().ok()?,
        })
    }

    fn watch<V: 'static>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
        changed: fn(&mut V, &mut Window, &mut Context<V>),
    ) -> Vec<Subscription> {
        vec![
            form::watch(&self.date, window, cx, changed),
            form::watch(&self.value, window, cx, changed),
            form::watch(&self.value_currency, window, cx, changed),
            form::watch(&self.tax_paid, window, cx, changed),
            form::watch(&self.tax_paid_currency, window, cx, changed),
            form::watch(&self.country, window, cx, changed),
        ]
    }

    fn render(&self, busy: bool, cx: &App) -> Div {
        form::grid()
            .child(form::date_field("Date", &self.date, busy, cx))
            .child(form::input_field("Ticker", &self.ticker, busy, cx))
            .child(form::amount_field(
                "Value",
                "Value currency",
                &self.value,
                &self.value_currency,
                busy,
                cx,
            ))
            .child(form::amount_field(
                "Tax paid",
                "Tax paid currency",
                &self.tax_paid,
                &self.tax_paid_currency,
                busy,
                cx,
            ))
            .child(form::input_field("Country code", &self.country, busy, cx))
            .child(form::input_field("Provider", &self.provider, busy, cx))
    }
}
