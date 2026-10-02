use crate::components::form::{self, ChoiceState};
use gpui_kit::{component::input::InputState, *};
use pitpls_app::use_case::interest::{CreateInterestInput, UpdateInterestInput};
use pitpls_core::{common::Currency, interest::CalculatedInterest};

pub enum InterestSubmission {
    Create(CreateInterestInput),
    Update(UpdateInterestInput),
}

pub struct InterestForm {
    existing_id: Option<String>,
    id: Entity<InputState>,
    date: Entity<InputState>,
    value: Entity<InputState>,
    value_currency: Entity<ChoiceState<Currency>>,
    provider: Entity<InputState>,
}

impl InterestForm {
    pub fn new(record: Option<&CalculatedInterest>, window: &mut Window, cx: &mut App) -> Self {
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
            "Edit interest"
        } else {
            "Add interest"
        }
    }

    pub fn submission(&self, cx: &App) -> Result<InterestSubmission, String> {
        match &self.existing_id {
            Some(_) => self.update_input(cx).map(InterestSubmission::Update),
            None => self.create_input(cx).map(InterestSubmission::Create),
        }
    }

    fn create_input(&self, cx: &App) -> Result<CreateInterestInput, String> {
        let id = form::text(&self.id, cx);
        Ok(CreateInterestInput {
            id: (!id.is_empty()).then_some(id),
            date: form::required(&self.date, "Date (YYYY-MM-DD)", cx)?,
            value: form::required(&self.value, "Value", cx)?,
            value_currency: form::selected(&self.value_currency, "Value currency", cx)?,
            provider: form::required(&self.provider, "Provider", cx)?,
        })
    }

    fn update_input(&self, cx: &App) -> Result<UpdateInterestInput, String> {
        Ok(UpdateInterestInput {
            id: self
                .existing_id
                .clone()
                .ok_or("No record selected for editing")?,
            date: form::required(&self.date, "Date (YYYY-MM-DD)", cx)?,
            value: form::required(&self.value, "Value", cx)?,
            value_currency: form::selected(&self.value_currency, "Value currency", cx)?,
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
            .child(form::input_field("Value", &self.value, busy, cx))
            .child(form::select_field(
                "Value currency",
                &self.value_currency,
                busy,
                cx,
            ))
            .child(form::input_field("Provider", &self.provider, busy, cx))
    }
}
