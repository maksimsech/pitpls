use super::{RecordForm, RecordKind, Submission};
use crate::{
    components::{
        form::{self, ChoiceState},
        records::{DetailGroup, RowDisplay},
        table::Column,
    },
    format::{DisplayText, amount, date, pln},
};
use chrono::NaiveDate;
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::dividend::{self, CreateDividendInput, UpdateDividendInput};
use pitpls_core::{common::Currency, dividend::CalculatedDividend};
use rust_decimal::Decimal;
use std::sync::Arc;

pub struct Dividends;

impl RecordKind for Dividends {
    type Record = CalculatedDividend;
    type Form = DividendForm;

    const NAME: &'static str = "Dividend";
    const TOTALS_TITLE: &'static str = "Dividend totals";
    const TOTAL_LABELS: &'static [&'static str] =
        &["Income (I-65)", "To pay (G-47)", "Paid (G-48)"];

    fn id(record: &CalculatedDividend) -> &str {
        &record.id
    }

    fn date(record: &CalculatedDividend) -> NaiveDate {
        record.date
    }

    fn columns() -> Vec<Column> {
        vec![
            Column::text("Date", 110.),
            Column::text("Ticker", 75.),
            Column::text("Provider", 120.),
            Column::number("Value", 135.),
            Column::number("Tax paid", 135.),
            Column::text("Country", 85.),
        ]
    }

    fn display(record: &CalculatedDividend) -> RowDisplay {
        let cells = vec![
            date(record.date),
            DisplayText::plain(record.ticker.clone()),
            DisplayText::plain(record.provider.clone()),
            amount(record.value),
            amount(record.tax_paid),
            DisplayText::plain(record.country.to_string()),
        ];
        let details = vec![
            DetailGroup {
                title: "Original amounts",
                fields: vec![
                    ("Original value", amount(record.value)),
                    ("Original tax paid", amount(record.tax_paid)),
                ],
            },
            DetailGroup {
                title: "Conversion",
                fields: vec![
                    ("NBP date", date(record.nbp_date)),
                    ("Calculated value", pln(record.calculated_value)),
                    ("Calculated tax paid", pln(record.calculated_tax_paid)),
                ],
            },
            DetailGroup {
                title: "Tax calculation",
                fields: vec![
                    ("Calculated to pay", pln(record.calculated_to_pay)),
                    ("Max tax paid", pln(record.max_tax_paid)),
                    ("Used tax paid", pln(record.used_tax_paid)),
                ],
            },
        ];
        RowDisplay { cells, details }
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

    fn first_input(&self) -> &Entity<InputState> {
        if self.existing_id.is_some() {
            &self.ticker
        } else {
            &self.id
        }
    }

    fn submission(&self, cx: &App) -> Result<DividendSubmission, String> {
        let date = form::selected_date(&self.date, "Date", cx)?;
        let ticker = form::required(&self.ticker, "Ticker", cx)?;
        let value = form::required(&self.value, "Value", cx)?;
        let value_currency = form::selected(&self.value_currency, "Value currency", cx)?;
        let tax_paid = form::required(&self.tax_paid, "Tax paid", cx)?;
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

    fn render(&self, busy: bool, cx: &App) -> Div {
        let editing = self.existing_id.is_some();
        div()
            .flex()
            .flex_wrap()
            .gap_4()
            .max_w(px(700.))
            .child(form::input_field("ID", &self.id, busy || editing, cx))
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
