use super::{RecordForm, RecordKind, Submission};
use crate::{
    components::{
        form::{self, ChoiceState},
        records::{DetailGroup, RowDisplay},
        table::Column,
    },
    format::{DisplayText, amount, date, money, pln},
};
use chrono::NaiveDate;
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::interest::{self, CreateInterestInput, UpdateInterestInput};
use pitpls_core::{common::Currency, interest::CalculatedInterest};
use rust_decimal::Decimal;
use std::sync::Arc;

pub struct Interests;

impl RecordKind for Interests {
    type Record = CalculatedInterest;
    type Form = InterestForm;

    const NAME: &'static str = "Interest";
    const TOTALS_TITLE: &'static str = "Interest totals";
    const TOTAL_LABELS: &'static [&'static str] = &["Income (I-65)", "To pay (G-47)"];

    fn id(record: &CalculatedInterest) -> &str {
        &record.id
    }

    fn date(record: &CalculatedInterest) -> NaiveDate {
        record.date
    }

    fn columns() -> Vec<Column> {
        vec![
            Column::text("Date", 110.),
            Column::text("Provider", 120.),
            Column::number("Value", 135.),
            Column::number("Calculated value", 135.),
            Column::number("To pay", 135.),
        ]
    }

    fn display(record: &CalculatedInterest) -> RowDisplay {
        let cells = vec![
            date(record.date),
            DisplayText::plain(record.provider.clone()),
            amount(record.value),
            money(record.calculated_value),
            money(record.to_pay),
        ];
        let details = vec![
            DetailGroup {
                title: "Original amounts",
                fields: vec![("Original value", amount(record.value))],
            },
            DetailGroup {
                title: "Conversion",
                fields: vec![
                    ("NBP date", date(record.nbp_date)),
                    ("Calculated value", pln(record.calculated_value)),
                ],
            },
            DetailGroup {
                title: "Tax calculation",
                fields: vec![("To pay", pln(record.to_pay))],
            },
        ];
        RowDisplay { cells, details }
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
