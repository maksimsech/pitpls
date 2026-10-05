use super::{RecordForm, RecordKind, Submission};
use crate::{
    components::{
        form::{self, Choice, ChoiceState},
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
use pitpls_app::use_case::crypto::{self, CreateCryptoInput, UpdateCryptoInput};
use pitpls_core::{
    common::Currency,
    crypto::{Action, CalculatedCrypto},
};
use rust_decimal::Decimal;
use std::sync::Arc;

pub struct Crypto;

impl RecordKind for Crypto {
    type Record = CalculatedCrypto;
    type Form = CryptoForm;

    const NAME: &'static str = "Crypto";
    const TOTALS_TITLE: &'static str = "Crypto totals";
    const TOTAL_LABELS: &'static [&'static str] = &["Income (E-36)", "Costs (E-37)"];

    fn id(record: &CalculatedCrypto) -> &str {
        &record.id
    }

    fn date(record: &CalculatedCrypto) -> NaiveDate {
        record.date
    }

    fn columns() -> Vec<Column> {
        vec![
            Column::text("Date", 110.),
            Column::text("Provider", 120.),
            Column::text("Action", 85.),
            Column::number("Value", 135.),
            Column::number("Fee", 135.),
            Column::number("Calculated value", 135.),
            Column::number("Calculated fee", 135.),
        ]
    }

    fn display(record: &CalculatedCrypto) -> RowDisplay {
        let cells = vec![
            date(record.date),
            DisplayText::plain(record.provider.clone()),
            DisplayText::plain(action_title(record.action)),
            amount(record.value),
            amount(record.fee),
            money(record.calculated_value),
            money(record.calculated_fee),
        ];
        let details = vec![
            DetailGroup {
                title: "Original amounts",
                fields: vec![
                    ("Original value", amount(record.value)),
                    ("Original fee", amount(record.fee)),
                ],
            },
            DetailGroup {
                title: "Conversion",
                fields: vec![
                    ("NBP date", date(record.nbp_date)),
                    ("Calculated value", pln(record.calculated_value)),
                    ("Calculated fee", pln(record.calculated_fee)),
                ],
            },
        ];
        RowDisplay { cells, details }
    }

    async fn load(
        app: Arc<pitpls_app::App>,
        year: Option<i32>,
    ) -> Result<(Vec<Decimal>, Vec<CalculatedCrypto>), String> {
        let data = crypto::load_cryptos(&app, year).await?;
        Ok((vec![data.income, data.costs], data.calculated))
    }

    async fn save(app: Arc<pitpls_app::App>, submission: CryptoSubmission) -> Result<(), String> {
        match submission {
            Submission::Create(input) => crypto::create_crypto(&app, input).await.map(drop),
            Submission::Update(input) => crypto::update_crypto(&app, input).await,
        }
    }

    async fn delete(app: Arc<pitpls_app::App>, ids: Vec<String>) -> Result<u64, String> {
        crypto::delete_cryptos(&app, ids).await
    }
}

fn action_title(action: Action) -> &'static str {
    match action {
        Action::FiatBuy => "Buy",
        Action::FiatSell => "Sell",
    }
}

type CryptoSubmission = Submission<CreateCryptoInput, UpdateCryptoInput>;

pub struct CryptoForm {
    existing_id: Option<String>,
    id: Entity<InputState>,
    date: Entity<DatePickerState>,
    action: Entity<ChoiceState<Action>>,
    value: Entity<InputState>,
    value_currency: Entity<ChoiceState<Currency>>,
    fee: Entity<InputState>,
    fee_currency: Entity<ChoiceState<Currency>>,
    provider: Entity<InputState>,
}

impl RecordForm for CryptoForm {
    type Record = CalculatedCrypto;
    type Submission = CryptoSubmission;

    fn new(record: Option<&CalculatedCrypto>, window: &mut Window, cx: &mut App) -> Self {
        let text = |value: fn(&CalculatedCrypto) -> String| record.map(value).unwrap_or_default();
        Self {
            existing_id: record.map(|record| record.id.clone()),
            id: form::optional_id(text(|record| record.id.clone()), window, cx),
            date: form::date_picker(
                record.map_or_else(|| chrono::Local::now().date_naive(), |record| record.date),
                window,
                cx,
            ),
            action: form::select(
                [Action::FiatBuy, Action::FiatSell]
                    .into_iter()
                    .map(|action| Choice::new(action, action_title(action)))
                    .collect(),
                record.map_or(Action::FiatBuy, |record| record.action),
                window,
                cx,
            ),
            value: form::input(text(|record| record.value.value.to_string()), window, cx),
            value_currency: form::currency(
                record.map_or(Currency::USD, |record| record.value.currency),
                window,
                cx,
            ),
            fee: form::input(
                record.map_or_else(|| "0".into(), |record| record.fee.value.to_string()),
                window,
                cx,
            ),
            fee_currency: form::currency(
                record.map_or(Currency::USD, |record| record.fee.currency),
                window,
                cx,
            ),
            provider: form::input(text(|record| record.provider.clone()), window, cx),
        }
    }

    fn title(&self) -> &'static str {
        if self.existing_id.is_some() {
            "Edit crypto"
        } else {
            "Add crypto"
        }
    }

    fn first_input(&self) -> &Entity<InputState> {
        if self.existing_id.is_some() {
            &self.value
        } else {
            &self.id
        }
    }

    fn submission(&self, cx: &App) -> Result<CryptoSubmission, String> {
        let date = form::selected_date(&self.date, "Date", cx)?;
        let action = form::selected(&self.action, "Action", cx)?;
        let value = form::required(&self.value, "Value", cx)?;
        let value_currency = form::selected(&self.value_currency, "Value currency", cx)?;
        let fee = form::required(&self.fee, "Fee", cx)?;
        let fee_currency = form::selected(&self.fee_currency, "Fee currency", cx)?;
        let provider = form::required(&self.provider, "Provider", cx)?;
        Ok(match self.existing_id.clone() {
            Some(id) => Submission::Update(UpdateCryptoInput {
                id,
                date,
                action,
                value,
                value_currency,
                fee,
                fee_currency,
                provider,
            }),
            None => Submission::Create(CreateCryptoInput {
                id: form::optional(&self.id, cx),
                date,
                action,
                value,
                value_currency,
                fee,
                fee_currency,
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
            .child(form::select_field("Action", &self.action, busy, cx))
            .child(form::amount_field(
                "Value",
                "Value currency",
                &self.value,
                &self.value_currency,
                busy,
                cx,
            ))
            .child(form::amount_field(
                "Fee",
                "Fee currency",
                &self.fee,
                &self.fee_currency,
                busy,
                cx,
            ))
            .child(form::input_field("Provider", &self.provider, busy, cx))
    }
}
