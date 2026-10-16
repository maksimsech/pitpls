use super::{RecordForm, RecordKind, Submission, conversion, date_label, day};
use crate::{
    components::{
        form::{self, Choice, ChoiceState},
        records::{CellStyle, Preview, RecordColumn, RowDisplay, Step, StepLine},
    },
    format::{DisplayText, amount, date, pln},
    navigation::Page,
    services::Error as ServiceError,
};
use chrono::{DateTime, Utc};
use gpui_kit::{
    component::{date_picker::DatePickerState, input::InputState},
    *,
};
use pitpls_app::use_case::{
    crypto::{self, CreateCryptoInput, CryptoFields, UpdateCryptoInput},
    year::YearInfo,
};
use pitpls_core::{
    common::{Amount, Currency},
    crypto::{self as core_crypto, Action, CalculatedCrypto},
};
use pitpls_importers::OutputType;
use rust_decimal::Decimal;
use rust_i18n::t;
use std::{borrow::Cow, sync::Arc};

pub struct Crypto;

impl RecordKind for Crypto {
    type Record = CalculatedCrypto;
    type Form = CryptoForm;

    const PAGE: Page = Page::Crypto;
    const EMPTY: &'static str = "crypto.empty";
    const EMPTY_YEAR: &'static str = "crypto.empty_year";
    const OTHER_YEAR: &'static str = "crypto.other_year";
    const TOTAL_LABELS: &'static [&'static str] = &["tax.income_e36", "tax.costs_e37"];

    fn id(record: &CalculatedCrypto) -> &str {
        &record.id
    }

    fn columns() -> Vec<RecordColumn> {
        vec![
            RecordColumn::new("column.date", CellStyle::Time, 100., 50.),
            RecordColumn::new("column.action", CellStyle::Tag, 64., 56.),
            RecordColumn::grow("column.provider", CellStyle::Muted),
            RecordColumn::new("column.value", CellStyle::Number, 150., 120.),
            RecordColumn::new("column.fee", CellStyle::Number, 104., 90.).optional(),
            RecordColumn::new("column.calculated_value", CellStyle::Number, 160., 124.),
            RecordColumn::new("column.calculated_fee", CellStyle::Number, 120., 100.).optional(),
        ]
    }

    fn display(record: &CalculatedCrypto) -> RowDisplay {
        let action = action_title(record.action);
        let cells = vec![
            day(record.time),
            DisplayText::plain(action.clone()),
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
                StepLine::Formula(vec![DisplayText::plain(t!("crypto.buy_formula"))]),
                StepLine::Result(vec![
                    DisplayText::plain(t!("tax.costs_e37")),
                    added(record.calculated_value + record.calculated_fee),
                ]),
                StepLine::Caption(t!("crypto.sale_note").into()),
            ],
            Action::FiatSell => vec![
                StepLine::Formula(vec![DisplayText::plain(t!("crypto.sell_formula"))]),
                StepLine::Result(vec![
                    DisplayText::plain(t!("tax.income_e36")),
                    added(record.calculated_value),
                ]),
                StepLine::Result(vec![
                    DisplayText::plain(t!("tax.costs_e37")),
                    added(record.calculated_fee),
                ]),
                StepLine::Caption(t!("crypto.purchase_note").into()),
            ],
        };
        let steps = vec![
            Step {
                title: "field.value",
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
                title: "field.fee",
                lines: vec![
                    StepLine::Formula(conversion(record.fee, record.fee_nbp_rate)),
                    StepLine::Result(vec![pln(record.calculated_fee)]),
                    StepLine::Caption(t!("column.calculated_fee").into()),
                ],
            },
            Step {
                title: "crypto.step_adds_to",
                lines: adds_to,
            },
        ];
        RowDisplay {
            id: record.id.clone().into(),
            time: record.time,
            cells,
            steps,
            search: format!("{}\n{action}", record.provider).to_lowercase(),
            label: format!("{action} {}", date_label(record.time)).into(),
            ..Default::default()
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
            parts.push((Some("records.subtotal_income"), income));
        }
        if buys || !costs.is_zero() {
            parts.push((Some("records.subtotal_costs"), costs));
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
    ) -> Result<(Vec<Decimal>, Vec<CalculatedCrypto>), ServiceError> {
        let data = crypto::load_cryptos(&app, year).await?;
        Ok((vec![data.income, data.costs], data.calculated))
    }

    async fn save(
        app: Arc<pitpls_app::App>,
        submission: CryptoSubmission,
    ) -> Result<(), ServiceError> {
        match submission {
            Submission::Create(input) => crypto::create_crypto(&app, input).await.map(drop),
            Submission::Update(input) => crypto::update_crypto(&app, input).await,
        }
        .map_err(ServiceError::from)
    }

    async fn delete(app: Arc<pitpls_app::App>, ids: Vec<String>) -> Result<u64, ServiceError> {
        crypto::delete_cryptos(&app, ids)
            .await
            .map_err(ServiceError::from)
    }

    async fn preview(
        app: Arc<pitpls_app::App>,
        draft: CryptoDraft,
    ) -> Result<CalculatedCrypto, ServiceError> {
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
            time: draft.time,
            provider: String::new(),
        };
        crypto::preview_crypto(&app, record)
            .await
            .map_err(ServiceError::from)
    }

    fn preview_display(record: &CalculatedCrypto) -> Preview {
        let mut results = vec![("column.calculated_fee", pln(record.calculated_fee))];
        match record.action {
            Action::FiatBuy => results.push((
                "tax.costs_e37",
                added(record.calculated_value + record.calculated_fee),
            )),
            Action::FiatSell => {
                results.push(("tax.income_e36", added(record.calculated_value)));
                results.push(("tax.costs_e37", added(record.calculated_fee)));
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
    time: DateTime<Utc>,
    action: Action,
    value: (Decimal, Currency),
    fee: (Decimal, Currency),
}

fn action_title(action: Action) -> Cow<'static, str> {
    match action {
        Action::FiatBuy => t!("action.buy"),
        Action::FiatSell => t!("action.sell"),
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
            date: form::date_picker(record.map(|record| record.time), window, cx),
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

    fn title(&self) -> Cow<'static, str> {
        if self.existing_id.is_some() {
            t!("crypto.edit")
        } else {
            t!("crypto.add")
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
        let time = form::selected_time(&self.date, &t!("field.date"), cx)?;
        let action = form::selected(&self.action, &t!("field.action"), cx)?;
        let value = form::amount(&self.value, &t!("field.value"), cx)?;
        let value_currency = form::selected(&self.value_currency, &t!("field.value_currency"), cx)?;
        let fee = form::amount(&self.fee, &t!("field.fee"), cx)?;
        let fee_currency = form::selected(&self.fee_currency, &t!("field.fee_currency"), cx)?;
        let provider = form::required(&self.provider, &t!("field.provider"), cx)?;
        let fields = CryptoFields {
            time,
            action,
            value,
            value_currency,
            fee,
            fee_currency,
            provider,
        };
        Ok(match self.existing_id.clone() {
            Some(id) => Submission::Update(UpdateCryptoInput { id, fields }),
            None => Submission::Create(CreateCryptoInput {
                id: form::optional(&self.id, cx),
                fields,
            }),
        })
    }

    fn draft(&self, cx: &App) -> Option<CryptoDraft> {
        Some(CryptoDraft {
            time: form::picked_time(&self.date, cx)?,
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
            .child(form::date_field(t!("field.date"), &self.date, busy, cx))
            .child(form::select_field(
                t!("field.action"),
                &self.action,
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
                t!("field.fee"),
                t!("field.fee_currency"),
                &self.fee,
                &self.fee_currency,
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
