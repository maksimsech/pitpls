use super::{RecordForm, RecordKind, Submission, conversion, date_label, day, percent};
use crate::{
    components::{
        form::{self, ChoiceState},
        records::{CellStyle, Preview, RecordColumn, RowDisplay, Step, StepLine},
    },
    format::{DisplayText, amount, date, money, pln},
    navigation::Page,
    services::Error as ServiceError,
};
use chrono::{DateTime, NaiveDate, Utc};
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::{
    interest::{self, CreateInterestInput, UpdateInterestInput},
    year::YearInfo,
};
use pitpls_core::{
    common::{Amount, Currency},
    interest::{CalculatedInterest, Interest},
    tax::POLAND_TAX,
};
use pitpls_importers::OutputType;
use portfolio::InterestPortfolioView;
use rust_decimal::Decimal;
use rust_i18n::t;
use std::{borrow::Cow, sync::Arc};

pub struct Interests;

impl RecordKind for Interests {
    type Record = CalculatedInterest;
    type Portfolio = InterestPortfolioView;
    type Form = InterestForm;

    const PAGE: Page = Page::Interests;
    const EMPTY: &'static str = "interests.empty";
    const EMPTY_YEAR: &'static str = "interests.empty_year";
    const OTHER_YEAR: &'static str = "interests.other_year";
    const TOTAL_LABELS: &'static [&'static str] = &["tax.income_i65", "tax.to_pay_short_g47"];

    fn id(record: &CalculatedInterest) -> &str {
        &record.id
    }

    fn time(record: &CalculatedInterest) -> DateTime<Utc> {
        record.time
    }

    fn columns() -> Vec<RecordColumn> {
        vec![
            RecordColumn::new("column.date", CellStyle::Time, 116., 50.),
            RecordColumn::grow("column.provider", CellStyle::Muted),
            RecordColumn::new("column.value", CellStyle::Number, 150., 104.),
            RecordColumn::new("column.calculated_value", CellStyle::Number, 170., 116.),
            RecordColumn::new("column.to_pay", CellStyle::Number, 140., 100.),
        ]
    }

    fn display(record: &CalculatedInterest) -> RowDisplay {
        let cells = vec![
            day(record.time),
            DisplayText::plain(record.provider.clone()),
            amount(record.value),
            pln(record.calculated_value),
            pln(record.to_pay),
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
                    StepLine::Result(vec![pln(record.to_pay)]),
                    StepLine::Caption(t!("column.to_pay").into()),
                ],
            },
        ];
        RowDisplay {
            id: record.id.clone().into(),
            time: record.time,
            cells,
            steps,
            search: record.provider.to_lowercase(),
            label: format!("{} {}", record.provider, date_label(record.time)).into(),
            ..Default::default()
        }
    }

    fn subtotal(records: &[&CalculatedInterest]) -> Vec<(Option<&'static str>, Decimal)> {
        let value = records.iter().map(|record| record.calculated_value).sum();
        let to_pay = records.iter().map(|record| record.to_pay).sum();
        vec![(None, value), (Some("records.subtotal_to_pay"), to_pay)]
    }

    fn portfolio_columns() -> Vec<RecordColumn> {
        let mut columns = Self::columns();
        columns.pop();
        columns
    }

    fn portfolio_display(record: &CalculatedInterest) -> RowDisplay {
        let mut row = Self::display(record);
        row.cells.pop();
        row.steps.truncate(1);
        row
    }

    fn portfolio_preview(record: &CalculatedInterest) -> Preview {
        let mut preview = Self::preview_display(record);
        preview.results.clear();
        preview
    }

    fn portfolio_subtotal(records: &[&CalculatedInterest]) -> Vec<(Option<&'static str>, Decimal)> {
        vec![(
            None,
            records.iter().map(|record| record.calculated_value).sum(),
        )]
    }

    async fn load_portfolio(
        app: Arc<pitpls_app::App>,
        today: NaiveDate,
    ) -> Result<(InterestPortfolioView, Vec<CalculatedInterest>), ServiceError> {
        let data = pitpls_app::use_case::portfolio::load_interest_portfolio(&app, today).await?;
        Ok((
            InterestPortfolioView::new(
                data.months,
                data.years,
                data.providers,
                data.last_12_months,
                today,
            ),
            data.data.calculated,
        ))
    }

    fn imported(output: &OutputType) -> bool {
        matches!(output, OutputType::Interest)
    }

    fn count(year: &YearInfo) -> u32 {
        year.interests
    }

    async fn load(
        app: Arc<pitpls_app::App>,
        year: Option<i32>,
    ) -> Result<(Vec<Decimal>, Vec<CalculatedInterest>), ServiceError> {
        let data = interest::load_interests(&app, year).await?;
        Ok((vec![data.income, data.to_pay], data.calculated))
    }

    async fn save(
        app: Arc<pitpls_app::App>,
        submission: InterestSubmission,
    ) -> Result<(), ServiceError> {
        match submission {
            Submission::Create(input) => interest::create_interest(&app, input).await.map(drop),
            Submission::Update(input) => interest::update_interest(&app, input).await,
        }
        .map_err(ServiceError::from)
    }

    async fn delete(app: Arc<pitpls_app::App>, ids: Vec<String>) -> Result<u64, ServiceError> {
        interest::delete_interests(&app, ids)
            .await
            .map_err(ServiceError::from)
    }

    async fn preview(
        app: Arc<pitpls_app::App>,
        draft: InterestDraft,
    ) -> Result<CalculatedInterest, ServiceError> {
        let (value, currency) = draft.value;
        let interest = Interest {
            id: String::new(),
            time: draft.time,
            value: Amount { value, currency },
            provider: String::new(),
        };
        interest::preview_interest(&app, interest)
            .await
            .map_err(ServiceError::from)
    }

    fn preview_display(record: &CalculatedInterest) -> Preview {
        Preview {
            nbp_date: record.nbp_date,
            formula: conversion(record.value, record.nbp_rate),
            value: pln(record.calculated_value),
            results: vec![("column.to_pay", pln(record.to_pay))],
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct InterestDraft {
    time: DateTime<Utc>,
    value: (Decimal, Currency),
}

type InterestSubmission = Submission<CreateInterestInput, UpdateInterestInput>;

pub struct InterestForm {
    existing_id: Option<String>,
    id: Entity<InputState>,
    date: Entity<DatePickerState>,
    value: Entity<InputState>,
    value_currency: Entity<ChoiceState<Currency>>,
    provider: Entity<InputState>,
}

impl RecordForm for InterestForm {
    type Record = CalculatedInterest;
    type Submission = InterestSubmission;
    type Draft = InterestDraft;

    fn new(record: Option<&CalculatedInterest>, window: &mut Window, cx: &mut App) -> Self {
        let text = |value: fn(&CalculatedInterest) -> String| record.map(value).unwrap_or_default();
        Self {
            existing_id: record.map(|record| record.id.clone()),
            id: form::optional_id(text(|record| record.id.clone()), window, cx),
            date: form::date_picker(record.map(|record| record.time), window, cx),
            value: form::input(text(|record| record.value.value.to_string()), window, cx),
            value_currency: form::currency(
                record.map_or(Currency::USD, |record| record.value.currency),
                window,
                cx,
            ),
            provider: form::input(text(|record| record.provider.clone()), window, cx),
        }
    }

    fn title(&self) -> Cow<'static, str> {
        if self.existing_id.is_some() {
            t!("interests.edit")
        } else {
            t!("interests.add")
        }
    }

    fn existing_id(&self) -> Option<&str> {
        self.existing_id.as_deref()
    }

    fn id_input(&self) -> &Entity<InputState> {
        &self.id
    }

    fn first_input(&self) -> &Entity<InputState> {
        &self.value
    }

    fn submission(&self, cx: &App) -> Result<InterestSubmission, String> {
        let time = form::selected_time(&self.date, &t!("field.date"), cx)?;
        let value = form::amount(&self.value, &t!("field.value"), cx)?;
        let value_currency = form::selected(&self.value_currency, &t!("field.value_currency"), cx)?;
        let provider = form::required(&self.provider, &t!("field.provider"), cx)?;
        Ok(match self.existing_id.clone() {
            Some(id) => Submission::Update(UpdateInterestInput {
                id,
                time,
                value,
                value_currency,
                provider,
            }),
            None => Submission::Create(CreateInterestInput {
                id: form::optional(&self.id, cx),
                time,
                value,
                value_currency,
                provider,
            }),
        })
    }

    fn draft(&self, cx: &App) -> Option<InterestDraft> {
        Some(InterestDraft {
            time: form::picked_time(&self.date, cx)?,
            value: form::parsed_amount(&self.value, &self.value_currency, cx)?,
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
        ]
    }

    fn render(&self, busy: bool, cx: &App) -> Div {
        form::grid()
            .child(form::date_field(t!("field.date"), &self.date, busy, cx))
            .child(form::amount_field(
                t!("field.value"),
                t!("field.value_currency"),
                &self.value,
                &self.value_currency,
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
mod portfolio;
