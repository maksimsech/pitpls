use super::{RecordForm, RecordKind, Submission, conversion, day};
use crate::{
    components::{
        form::{self, Choice, ChoiceState},
        records::{CellStyle, Preview, RecordColumn, RowDisplay, Step, StepLine},
    },
    format::{DisplayText, amount, date, pln},
    navigation::Page,
};
use chrono::NaiveDate;
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::{
    crypto::{self, CreateCryptoInput, UpdateCryptoInput},
    year::YearInfo,
};
use pitpls_core::{
    common::{Amount, Currency},
    crypto::{self as core_crypto, Action, CalculatedCrypto},
};
use pitpls_importers::OutputType;
use rust_decimal::Decimal;
use std::sync::Arc;

pub struct Crypto;

impl RecordKind for Crypto {
    type Record = CalculatedCrypto;
    type Form = CryptoForm;

    const PAGE: Page = Page::Crypto;
    const NAME: &'static str = "Crypto";
    const PLURAL: &'static str = "crypto";
    const TOTAL_LABELS: &'static [&'static str] = &["Income (E-36)", "Costs (E-37)"];

    fn id(record: &CalculatedCrypto) -> &str {
        &record.id
    }

    fn date(record: &CalculatedCrypto) -> NaiveDate {
        record.date
    }

    fn columns() -> Vec<RecordColumn> {
        vec![
            RecordColumn::new("Date", CellStyle::Muted, 60., 50.),
            RecordColumn::new("Action", CellStyle::Tag, 64., 56.),
            RecordColumn::grow("Provider", CellStyle::Muted),
            RecordColumn::new("Value", CellStyle::Number, 150., 120.),
            RecordColumn::new("Fee", CellStyle::Number, 104., 90.).optional(),
            RecordColumn::new("Calculated value", CellStyle::Number, 160., 124.),
            RecordColumn::new("Calculated fee", CellStyle::Number, 120., 100.).optional(),
        ]
    }

    fn display(record: &CalculatedCrypto) -> RowDisplay {
        let action = action_title(record.action);
        let cells = vec![
            day(record.date),
            DisplayText::plain(action),
            DisplayText::plain(record.provider.clone()),
            amount(record.value),
            amount(record.fee),
            pln(record.calculated_value),
            pln(record.calculated_fee),
        ];
        // Where the calculation adds the record, as in
        // `calculate_sell_buy_values`; the sum is shown, never fed back.
        let adds_to = match record.action {
            Action::FiatBuy => vec![
                StepLine::Formula(vec![DisplayText::plain("Buy · value + fee")]),
                StepLine::Result(vec![
                    DisplayText::plain("Costs (E-37)"),
                    added(record.calculated_value + record.calculated_fee),
                ]),
                StepLine::Caption(
                    "A sale adds its value to Income (E-36) and its fee to Costs (E-37)".into(),
                ),
            ],
            Action::FiatSell => vec![
                StepLine::Formula(vec![DisplayText::plain(
                    "Sell · value to income, fee to costs",
                )]),
                StepLine::Result(vec![
                    DisplayText::plain("Income (E-36)"),
                    added(record.calculated_value),
                ]),
                StepLine::Result(vec![
                    DisplayText::plain("Costs (E-37)"),
                    added(record.calculated_fee),
                ]),
                StepLine::Caption("A purchase adds its value and fee to Costs (E-37)".into()),
            ],
        };
        let steps = vec![
            Step {
                title: "Value",
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
                title: "Fee",
                lines: vec![
                    StepLine::Formula(conversion(record.fee, record.fee_nbp_rate)),
                    StepLine::Result(vec![pln(record.calculated_fee)]),
                    StepLine::Caption("Calculated fee".into()),
                ],
            },
            Step {
                title: "Adds to",
                lines: adds_to,
            },
        ];
        RowDisplay {
            cells,
            steps,
            search: format!("{}\n{action}", record.provider).to_lowercase(),
            label: format!("{action} {}", date(record.date).main).into(),
        }
    }

    /// A part shows only when the month has records for it.
    fn subtotal(records: &[&CalculatedCrypto]) -> Vec<(Option<&'static str>, Decimal)> {
        let (mut income, mut costs) = (Decimal::ZERO, Decimal::ZERO);
        let (mut sells, mut buys) = (false, false);
        for record in records {
            match record.action {
                Action::FiatBuy => {
                    buys = true;
                    costs += record.calculated_value + record.calculated_fee;
                }
                Action::FiatSell => {
                    sells = true;
                    income += record.calculated_value;
                    costs += record.calculated_fee;
                }
            }
        }
        let mut parts = vec![];
        if sells {
            parts.push((Some("income"), income));
        }
        if buys || !costs.is_zero() {
            parts.push((Some("costs"), costs));
        }
        parts
    }

    fn imported(output: &OutputType) -> bool {
        matches!(output, OutputType::Crypto)
    }

    fn count(year: &YearInfo) -> u32 {
        year.cryptos
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

    async fn preview(
        app: Arc<pitpls_app::App>,
        draft: CryptoDraft,
    ) -> Result<CalculatedCrypto, String> {
        let (value, value_currency) = draft.value;
        let (fee, fee_currency) = draft.fee;
        let record = core_crypto::Crypto {
            id: String::new(),
            value: Amount {
                value,
                currency: value_currency,
            },
            fee: Amount {
                value: fee,
                currency: fee_currency,
            },
            action: draft.action,
            date: draft.date,
            provider: String::new(),
        };
        crypto::preview_crypto(&app, record).await
    }

    fn preview_display(record: &CalculatedCrypto) -> Preview {
        let mut results = vec![("Calculated fee", pln(record.calculated_fee))];
        match record.action {
            Action::FiatBuy => results.push((
                "Costs (E-37)",
                added(record.calculated_value + record.calculated_fee),
            )),
            Action::FiatSell => {
                results.push(("Income (E-36)", added(record.calculated_value)));
                results.push(("Costs (E-37)", added(record.calculated_fee)));
            }
        }
        Preview {
            nbp_date: record.nbp_date,
            formula: conversion(record.value, record.nbp_rate),
            value: pln(record.calculated_value),
            results,
        }
    }
}

fn added(value: Decimal) -> DisplayText {
    let value = pln(value);
    DisplayText {
        main: format!("+{}", value.main).into(),
        ..value
    }
}

#[derive(Clone, PartialEq)]
pub struct CryptoDraft {
    date: NaiveDate,
    action: Action,
    value: (Decimal, Currency),
    fee: (Decimal, Currency),
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
    type Draft = CryptoDraft;

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

    fn existing_id(&self) -> Option<&str> {
        self.existing_id.as_deref()
    }

    fn id_input(&self) -> &Entity<InputState> {
        &self.id
    }

    fn first_input(&self) -> &Entity<InputState> {
        &self.value
    }

    fn submission(&self, cx: &App) -> Result<CryptoSubmission, String> {
        let date = form::selected_date(&self.date, "Date", cx)?;
        let action = form::selected(&self.action, "Action", cx)?;
        let value = form::amount(&self.value, "Value", cx)?;
        let value_currency = form::selected(&self.value_currency, "Value currency", cx)?;
        let fee = form::amount(&self.fee, "Fee", cx)?;
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

    fn draft(&self, cx: &App) -> Option<CryptoDraft> {
        Some(CryptoDraft {
            date: form::picked_date(&self.date, cx)?,
            action: self.action.read(cx).selected_value().copied()?,
            value: form::parsed_amount(&self.value, &self.value_currency, cx)?,
            fee: form::parsed_amount(&self.fee, &self.fee_currency, cx)?,
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
            form::watch(&self.action, window, cx, changed),
            form::watch(&self.value, window, cx, changed),
            form::watch(&self.value_currency, window, cx, changed),
            form::watch(&self.fee, window, cx, changed),
            form::watch(&self.fee_currency, window, cx, changed),
        ]
    }

    fn render(&self, busy: bool, cx: &App) -> Div {
        form::grid()
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
