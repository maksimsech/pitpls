use super::{RecordForm, RecordKind, Submission, conversion, date_label, day, percent};
use crate::{
    components::{
        form::{self, ChoiceState},
        records::{CellStyle, EntryStyle, Preview, RecordColumn, RowDisplay, Step, StepLine},
    },
    format::{DisplayText, amount, date, money, pln},
    navigation::Page,
    services::Error as ServiceError,
};
use chrono::{DateTime, Utc};
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::{
    dividend::{self, CreateDividendInput, DividendFields, UpdateDividendInput},
    year::YearInfo,
};
use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::{CalculatedDividend, Dividend},
    tax::{POLAND_TAX, get_treaty_tax},
};
use pitpls_importers::OutputType;
use rust_decimal::Decimal;
use rust_i18n::t;
use std::{borrow::Cow, sync::Arc};

pub struct Dividends;

impl RecordKind for Dividends {
    type Record = CalculatedDividend;
    type Form = DividendForm;

    const PAGE: Page = Page::Dividends;
    const EMPTY: &'static str = "dividends.empty";
    const EMPTY_YEAR: &'static str = "dividends.empty_year";
    const OTHER_YEAR: &'static str = "dividends.other_year";
    const TOTAL_LABELS: &'static [&'static str] = &[
        "tax.income_i65",
        "tax.to_pay_short_g47",
        "tax.paid_short_g48",
    ];

    fn id(record: &CalculatedDividend) -> &str {
        &record.id
    }

    fn columns() -> Vec<RecordColumn> {
        vec![
            RecordColumn::new("column.date", CellStyle::Time, 104., 44.),
            RecordColumn::new("column.ticker", CellStyle::Strong, 76., 56.),
            RecordColumn::grow("column.provider", CellStyle::Muted),
            RecordColumn::new("column.country", CellStyle::Muted, 64., 46.),
            RecordColumn::new("column.value", CellStyle::Number, 150., 124.),
            RecordColumn::new("column.tax_paid", CellStyle::Number, 124., 104.),
        ]
    }

    fn display(record: &CalculatedDividend) -> RowDisplay {
        let cells = vec![
            day(record.time),
            DisplayText::plain(record.ticker.clone()),
            DisplayText::plain(record.provider.clone()),
            DisplayText::plain(record.country.to_string()),
            amount(record.value),
            amount(record.tax_paid),
        ];
        let steps = vec![
            Step {
                title: "records.step_conversion",
                lines: vec![
                    StepLine::Formula(conversion(record.value, record.nbp_rate)),
                    StepLine::Result(vec![pln(record.calculated_value)]),
                    StepLine::Caption(
                        t!(
                            "records.calculated_value_nbp",
                            date = date(record.nbp_date).main
                        )
                        .into(),
                    ),
                ],
            },
            Step {
                title: "records.step_polish_tax",
                lines: vec![
                    StepLine::Formula(vec![
                        DisplayText::plain(format!("{} ×", percent(POLAND_TAX))),
                        money(record.calculated_value),
                    ]),
                    StepLine::Result(vec![pln(record.calculated_to_pay)]),
                    StepLine::Caption(t!("dividends.calculated_to_pay").into()),
                ],
            },
            Step {
                title: "dividends.step_credit",
                lines: vec![
                    StepLine::Formula(
                        [
                            vec![DisplayText::plain(t!("field.tax_paid"))],
                            conversion(record.tax_paid, record.tax_paid_nbp_rate),
                        ]
                        .concat(),
                    ),
                    StepLine::Entry {
                        label: t!("dividends.calculated_tax_paid").into(),
                        value: vec![pln(record.calculated_tax_paid)],
                        style: EntryStyle::Normal,
                    },
                    StepLine::Entry {
                        label: t!("dividends.max_tax_paid").into(),
                        value: vec![pln(record.max_tax_paid)],
                        style: EntryStyle::Normal,
                    },
                    StepLine::Entry {
                        label: t!("dividends.used_tax_paid").into(),
                        value: vec![pln(record.used_tax_paid)],
                        style: EntryStyle::Strong,
                    },
                    StepLine::Caption(
                        t!(
                            "dividends.treaty_note",
                            rate = percent(get_treaty_tax(&record.country)),
                            country = record.country
                        )
                        .into(),
                    ),
                ],
            },
        ];
        RowDisplay {
            id: record.id.clone().into(),
            time: record.time,
            cells,
            steps,
            search: format!("{}\n{}\n{}", record.ticker, record.provider, record.country)
                .to_lowercase(),
            label: format!("{} {}", record.ticker, date_label(record.time)).into(),
            ..Default::default()
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
    ) -> Result<(Vec<Decimal>, Vec<CalculatedDividend>), ServiceError> {
        let data = dividend::load_dividends(&app, year).await?;
        Ok((vec![data.income, data.to_pay, data.paid], data.calculated))
    }

    async fn save(
        app: Arc<pitpls_app::App>,
        submission: DividendSubmission,
    ) -> Result<(), ServiceError> {
        match submission {
            Submission::Create(input) => dividend::create_dividend(&app, input).await.map(drop),
            Submission::Update(input) => dividend::update_dividend(&app, input).await,
        }
        .map_err(ServiceError::from)
    }

    async fn delete(app: Arc<pitpls_app::App>, ids: Vec<String>) -> Result<u64, ServiceError> {
        dividend::delete_dividends(&app, ids)
            .await
            .map_err(ServiceError::from)
    }

    async fn preview(
        app: Arc<pitpls_app::App>,
        draft: DividendDraft,
    ) -> Result<CalculatedDividend, ServiceError> {
        let (value, value_currency) = draft.value;
        let (tax_paid, tax_paid_currency) = draft.tax_paid;
        let dividend = Dividend {
            id: String::new(),
            time: draft.time,
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
        dividend::preview_dividend(&app, dividend)
            .await
            .map_err(ServiceError::from)
    }

    fn preview_display(record: &CalculatedDividend) -> Preview {
        Preview {
            nbp_date: record.nbp_date,
            formula: conversion(record.value, record.nbp_rate),
            value: pln(record.calculated_value),
            results: vec![
                ("dividends.calculated_to_pay", pln(record.calculated_to_pay)),
                ("dividends.used_tax_paid", pln(record.used_tax_paid)),
            ],
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct DividendDraft {
    time: DateTime<Utc>,
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
            date: form::date_picker(record.map(|record| record.time), window, cx),
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

    fn title(&self) -> Cow<'static, str> {
        if self.existing_id.is_some() {
            t!("dividends.edit")
        } else {
            t!("dividends.add")
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
        let time = form::selected_time(&self.date, &t!("field.date"), cx)?;
        let ticker = form::required(&self.ticker, &t!("field.ticker"), cx)?;
        let value = form::amount(&self.value, &t!("field.value"), cx)?;
        let value_currency = form::selected(&self.value_currency, &t!("field.value_currency"), cx)?;
        let tax_paid = form::amount(&self.tax_paid, &t!("field.tax_paid"), cx)?;
        let tax_paid_currency =
            form::selected(&self.tax_paid_currency, &t!("field.tax_paid_currency"), cx)?;
        let country = form::required(&self.country, &t!("field.country_code"), cx)?
            .parse()
            .map_err(crate::messages::country_error)?;
        let provider = form::required(&self.provider, &t!("field.provider"), cx)?;
        let fields = DividendFields {
            time,
            ticker,
            value,
            value_currency,
            tax_paid,
            tax_paid_currency,
            country,
            provider,
        };
        Ok(match self.existing_id.clone() {
            Some(id) => Submission::Update(UpdateDividendInput { id, fields }),
            None => Submission::Create(CreateDividendInput {
                id: form::optional(&self.id, cx),
                fields,
            }),
        })
    }

    fn draft(&self, cx: &App) -> Option<DividendDraft> {
        Some(DividendDraft {
            time: form::picked_time(&self.date, cx)?,
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
            .child(form::date_field(t!("field.date"), &self.date, busy, cx))
            .child(form::input_field(
                t!("field.ticker"),
                &self.ticker,
                busy,
                cx,
            ))
            .child(form::amount_field(
                t!("field.value"),
                t!("field.value_currency"),
                &self.value,
                &self.value_currency,
                busy,
                cx,
            ))
            .child(form::amount_field(
                t!("field.tax_paid"),
                t!("field.tax_paid_currency"),
                &self.tax_paid,
                &self.tax_paid_currency,
                busy,
                cx,
            ))
            .child(form::input_field(
                t!("field.country_code"),
                &self.country,
                busy,
                cx,
            ))
            .child(form::input_field(
                t!("field.provider"),
                &self.provider,
                busy,
                cx,
            ))
    }
}
