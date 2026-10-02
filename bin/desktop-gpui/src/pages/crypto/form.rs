use crate::components::form::{self, Choice, ChoiceState};
use gpui_kit::{component::input::InputState, *};
use pitpls_app::use_case::crypto::{CreateCryptoInput, UpdateCryptoInput};
use pitpls_core::{
    common::Currency,
    crypto::{Action, CalculatedCrypto},
};

pub enum CryptoSubmission {
    Create(CreateCryptoInput),
    Update(UpdateCryptoInput),
}

pub struct CryptoForm {
    existing_id: Option<String>,
    id: Entity<InputState>,
    date: Entity<InputState>,
    action: Entity<ChoiceState<Action>>,
    value: Entity<InputState>,
    value_currency: Entity<ChoiceState<Currency>>,
    fee: Entity<InputState>,
    fee_currency: Entity<ChoiceState<Currency>>,
    provider: Entity<InputState>,
}

impl CryptoForm {
    pub fn new(record: Option<&CalculatedCrypto>, window: &mut Window, cx: &mut App) -> Self {
        Self {
            existing_id: record.map(|record| record.id.clone()),
            id: form::input(
                record.map(|record| record.id.clone()).unwrap_or_default(),
                window,
                cx,
            ),
            date: form::input(
                record
                    .map(|record| record.date.to_string())
                    .unwrap_or_else(|| chrono::Local::now().date_naive().to_string()),
                window,
                cx,
            ),
            action: form::select(
                vec![
                    Choice::new(Action::FiatBuy, "Buy"),
                    Choice::new(Action::FiatSell, "Sell"),
                ],
                record
                    .map(|record| record.action)
                    .unwrap_or(Action::FiatBuy),
                window,
                cx,
            ),
            value: form::input(
                record
                    .map(|record| record.value.value.to_string())
                    .unwrap_or_default(),
                window,
                cx,
            ),
            value_currency: form::currency(
                record
                    .map(|record| record.value.currency)
                    .unwrap_or(Currency::USD),
                window,
                cx,
            ),
            fee: form::input(
                record
                    .map(|record| record.fee.value.to_string())
                    .unwrap_or_else(|| "0".into()),
                window,
                cx,
            ),
            fee_currency: form::currency(
                record
                    .map(|record| record.fee.currency)
                    .unwrap_or(Currency::USD),
                window,
                cx,
            ),
            provider: form::input(
                record
                    .map(|record| record.provider.clone())
                    .unwrap_or_default(),
                window,
                cx,
            ),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        let state = if self.existing_id.is_some() {
            &self.date
        } else {
            &self.id
        };
        window.focus(&state.focus_handle(cx), cx);
    }

    pub fn title(&self) -> &'static str {
        if self.existing_id.is_some() {
            "Edit crypto"
        } else {
            "Add crypto"
        }
    }

    pub fn submission(&self, cx: &App) -> Result<CryptoSubmission, String> {
        match &self.existing_id {
            Some(_) => self.update_input(cx).map(CryptoSubmission::Update),
            None => self.create_input(cx).map(CryptoSubmission::Create),
        }
    }

    fn create_input(&self, cx: &App) -> Result<CreateCryptoInput, String> {
        let id = form::text(&self.id, cx);
        Ok(CreateCryptoInput {
            id: (!id.is_empty()).then_some(id),
            date: form::required(&self.date, "Date (YYYY-MM-DD)", cx)?,
            action: form::selected(&self.action, "Action", cx)?,
            value: form::required(&self.value, "Value", cx)?,
            value_currency: form::selected(&self.value_currency, "Value currency", cx)?,
            fee: form::required(&self.fee, "Fee", cx)?,
            fee_currency: form::selected(&self.fee_currency, "Fee currency", cx)?,
            provider: form::required(&self.provider, "Provider", cx)?,
        })
    }

    fn update_input(&self, cx: &App) -> Result<UpdateCryptoInput, String> {
        Ok(UpdateCryptoInput {
            id: self
                .existing_id
                .clone()
                .ok_or("No record selected for editing")?,
            date: form::required(&self.date, "Date (YYYY-MM-DD)", cx)?,
            action: form::selected(&self.action, "Action", cx)?,
            value: form::required(&self.value, "Value", cx)?,
            value_currency: form::selected(&self.value_currency, "Value currency", cx)?,
            fee: form::required(&self.fee, "Fee", cx)?,
            fee_currency: form::selected(&self.fee_currency, "Fee currency", cx)?,
            provider: form::required(&self.provider, "Provider", cx)?,
        })
    }

    pub fn render(&self, busy: bool, cx: &App) -> Div {
        let editing = self.existing_id.is_some();
        div()
            .flex()
            .flex_wrap()
            .gap_4()
            .max_w(px(700.))
            .child(form::input_field(
                if editing {
                    "ID"
                } else {
                    "ID (optional; generated when blank)"
                },
                &self.id,
                busy || editing,
                cx,
            ))
            .child(form::input_field("Date (YYYY-MM-DD)", &self.date, busy, cx))
            .child(form::select_field("Action", &self.action, busy, cx))
            .child(form::input_field("Value", &self.value, busy, cx))
            .child(form::select_field(
                "Value currency",
                &self.value_currency,
                busy,
                cx,
            ))
            .child(form::input_field("Fee", &self.fee, busy, cx))
            .child(form::select_field(
                "Fee currency",
                &self.fee_currency,
                busy,
                cx,
            ))
            .child(form::input_field("Provider", &self.provider, busy, cx))
    }
}
