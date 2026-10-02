use crate::components::form::{self, ChoiceState};
use gpui_kit::{component::input::InputState, *};
use pitpls_app::use_case::dividend::{CreateDividendInput, UpdateDividendInput};
use pitpls_core::{common::Currency, dividend::CalculatedDividend};

pub enum DividendSubmission {
    Create(CreateDividendInput),
    Update(UpdateDividendInput),
}

pub struct DividendForm {
    existing_id: Option<String>,
    id: Entity<InputState>,
    date: Entity<InputState>,
    ticker: Entity<InputState>,
    value: Entity<InputState>,
    value_currency: Entity<ChoiceState<Currency>>,
    tax_paid: Entity<InputState>,
    tax_paid_currency: Entity<ChoiceState<Currency>>,
    country: Entity<InputState>,
    provider: Entity<InputState>,
}

impl DividendForm {
    pub fn new(record: Option<&CalculatedDividend>, window: &mut Window, cx: &mut App) -> Self {
        Self {
            existing_id: record.map(|record| record.id.clone()),
            id: form::optional_id(
                record.map(|record| record.id.clone()).unwrap_or_default(),
                window,
                cx,
            ),
            date: form::date_input(
                record
                    .map(|record| record.date.to_string())
                    .unwrap_or_else(|| chrono::Local::now().date_naive().to_string()),
                window,
                cx,
            ),
            ticker: form::input(
                record
                    .map(|record| record.ticker.clone())
                    .unwrap_or_default(),
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
            tax_paid: form::optional_id(
                record
                    .map(|record| record.tax_paid.value.to_string())
                    .unwrap_or_else(|| "0".into()),
                window,
                cx,
            ),
            tax_paid_currency: form::currency(
                record
                    .map(|record| record.tax_paid.currency)
                    .unwrap_or(Currency::USD),
                window,
                cx,
            ),
            country: form::input(
                record
                    .map(|record| record.country.to_string())
                    .unwrap_or_else(|| "US".into()),
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
            "Edit dividend"
        } else {
            "Add dividend"
        }
    }

    pub fn submission(&self, cx: &App) -> Result<DividendSubmission, String> {
        match &self.existing_id {
            Some(_) => self.update_input(cx).map(DividendSubmission::Update),
            None => self.create_input(cx).map(DividendSubmission::Create),
        }
    }

    fn create_input(&self, cx: &App) -> Result<CreateDividendInput, String> {
        let id = form::text(&self.id, cx);
        Ok(CreateDividendInput {
            id: (!id.is_empty()).then_some(id),
            date: form::required(&self.date, "Date", cx)?,
            ticker: form::required(&self.ticker, "Ticker", cx)?,
            value: form::required(&self.value, "Value", cx)?,
            value_currency: form::selected(&self.value_currency, "Value currency", cx)?,
            tax_paid: form::required(&self.tax_paid, "Tax paid", cx)?,
            tax_paid_currency: form::selected(&self.tax_paid_currency, "Tax paid currency", cx)?,
            country: form::required(&self.country, "Country code", cx)?.parse()?,
            provider: form::required(&self.provider, "Provider", cx)?,
        })
    }

    fn update_input(&self, cx: &App) -> Result<UpdateDividendInput, String> {
        Ok(UpdateDividendInput {
            id: self
                .existing_id
                .clone()
                .ok_or("No record selected for editing")?,
            date: form::required(&self.date, "Date", cx)?,
            ticker: form::required(&self.ticker, "Ticker", cx)?,
            value: form::required(&self.value, "Value", cx)?,
            value_currency: form::selected(&self.value_currency, "Value currency", cx)?,
            tax_paid: form::required(&self.tax_paid, "Tax paid", cx)?,
            tax_paid_currency: form::selected(&self.tax_paid_currency, "Tax paid currency", cx)?,
            country: form::required(&self.country, "Country code", cx)?.parse()?,
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
            .child(form::input_field("ID", &self.id, busy || editing, cx))
            .child(form::input_field("Date", &self.date, busy, cx))
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
