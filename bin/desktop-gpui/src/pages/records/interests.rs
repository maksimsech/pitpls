use super::{RecordForm, RecordKind, Submission, conversion, day, percent};
use crate::navigation::Page;
use crate::{
    components::{
        form::{self, ChoiceState},
        records::{CellStyle, RecordColumn, RowDisplay, Step, StepLine},
    },
    format::{DisplayText, amount, date, money, pln},
};
use chrono::NaiveDate;
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::{
    interest::{self, CreateInterestInput, UpdateInterestInput},
    year::YearInfo,
};
use pitpls_core::{common::Currency, interest::CalculatedInterest, tax::POLAND_TAX};
use pitpls_importers::OutputType;
use rust_decimal::Decimal;
use std::sync::Arc;

pub struct Interests;

impl RecordKind for Interests {
    type Record = CalculatedInterest;
    type Form = InterestForm;

    const PAGE: Page = Page::Interests;
    const NAME: &'static str = "Interest";
    const PLURAL: &'static str = "interest";
    const TOTAL_LABELS: &'static [&'static str] = &["Income (I-65)", "To pay (G-47)"];

    fn id(record: &CalculatedInterest) -> &str {
        &record.id
    }

    fn date(record: &CalculatedInterest) -> NaiveDate {
        record.date
    }

    fn columns() -> Vec<RecordColumn> {
        vec![
            RecordColumn::new("Date", CellStyle::Muted, 76., 50.),
            RecordColumn::grow("Provider", CellStyle::Muted),
            RecordColumn::new("Value", CellStyle::Number, 150., 104.),
            RecordColumn::new("Calculated value", CellStyle::Number, 170., 116.),
            RecordColumn::new("To pay", CellStyle::Number, 140., 100.),
        ]
    }

    fn display(record: &CalculatedInterest) -> RowDisplay {
        let cells = vec![
            day(record.date),
            DisplayText::plain(record.provider.clone()),
            amount(record.value),
            pln(record.calculated_value),
            pln(record.to_pay),
        ];
        let steps = vec![
            Step {
                title: "Conversion",
                lines: vec![
                    StepLine::Formula(conversion(record.value, record.nbp_rate).into()),
                    StepLine::Result(pln(record.calculated_value).full),
                    StepLine::Caption(
                        format!("Calculated value · NBP date {}", date(record.nbp_date).main)
                            .into(),
                    ),
                ],
            },
            Step {
                title: "Polish tax",
                lines: vec![
                    StepLine::Formula(
                        format!(
                            "{} × {}",
                            percent(POLAND_TAX),
                            money(record.calculated_value).full
                        )
                        .into(),
                    ),
                    StepLine::Result(pln(record.to_pay).full),
                    StepLine::Caption("To pay".into()),
                ],
            },
        ];
        RowDisplay {
            cells,
            steps,
            search: record.provider.to_lowercase(),
            label: format!("{} {}", record.provider, date(record.date).main).into(),
        }
    }

    fn subtotal(records: &[&CalculatedInterest]) -> Vec<(Option<&'static str>, Decimal)> {
        let value = records.iter().map(|record| record.calculated_value).sum();
        let to_pay = records.iter().map(|record| record.to_pay).sum();
        vec![(None, value), (Some("to pay"), to_pay)]
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
    ) -> Result<(Vec<Decimal>, Vec<CalculatedInterest>), String> {
        let data = interest::load_interests(&app, year).await?;
        Ok((vec![data.income, data.to_pay], data.calculated))
    }

    async fn save(app: Arc<pitpls_app::App>, submission: InterestSubmission) -> Result<(), String> {
        match submission {
            Submission::Create(input) => interest::create_interest(&app, input).await.map(drop),
            Submission::Update(input) => interest::update_interest(&app, input).await,
        }
    }

    async fn delete(app: Arc<pitpls_app::App>, ids: Vec<String>) -> Result<u64, String> {
        interest::delete_interests(&app, ids).await
    }
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

    fn new(record: Option<&CalculatedInterest>, window: &mut Window, cx: &mut App) -> Self {
        let text = |value: fn(&CalculatedInterest) -> String| record.map(value).unwrap_or_default();
        Self {
            existing_id: record.map(|record| record.id.clone()),
            id: form::optional_id(text(|record| record.id.clone()), window, cx),
            date: form::date_picker(
                record.map_or_else(|| chrono::Local::now().date_naive(), |record| record.date),
                window,
                cx,
            ),
            value: form::input(text(|record| record.value.value.to_string()), window, cx),
            value_currency: form::currency(
                record.map_or(Currency::USD, |record| record.value.currency),
                window,
                cx,
            ),
            provider: form::input(text(|record| record.provider.clone()), window, cx),
        }
    }

    fn title(&self) -> &'static str {
        if self.existing_id.is_some() {
            "Edit interest"
        } else {
            "Add interest"
        }
    }

    fn first_input(&self) -> &Entity<InputState> {
        if self.existing_id.is_some() {
            &self.value
        } else {
            &self.id
        }
    }

    fn submission(&self, cx: &App) -> Result<InterestSubmission, String> {
        let date = form::selected_date(&self.date, "Date", cx)?;
        let value = form::required(&self.value, "Value", cx)?;
        let value_currency = form::selected(&self.value_currency, "Value currency", cx)?;
        let provider = form::required(&self.provider, "Provider", cx)?;
        Ok(match self.existing_id.clone() {
            Some(id) => Submission::Update(UpdateInterestInput {
                id,
                date,
                value,
                value_currency,
                provider,
            }),
            None => Submission::Create(CreateInterestInput {
                id: form::optional(&self.id, cx),
                date,
                value,
                value_currency,
                provider,
            }),
        })
    }

    fn render(&self, busy: bool, cx: &App) -> Div {
        let editing = self.existing_id.is_some();
        div()
            .flex()
            .flex_wrap()
            .gap_4()
            .max_w(px(700.))
            .child(form::input_field("ID", &self.id, busy || editing, cx))
            .child(form::date_field("Date", &self.date, busy, cx))
            .child(form::amount_field(
                "Value",
                "Value currency",
                &self.value,
                &self.value_currency,
                busy,
                cx,
            ))
            .child(form::input_field("Provider", &self.provider, busy, cx))
    }
}
