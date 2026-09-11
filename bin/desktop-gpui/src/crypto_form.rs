//! Crypto form state and conversion to application-layer command inputs.

use chrono::Local;
use gpui::{App, AppContext, Context, Entity, Window};
use gpui_component::{
    IndexPath,
    input::InputState,
    select::{SearchableVec, SelectState},
};
use pitpls_app::use_case::crypto::{CreateCryptoInput, UpdateCryptoInput};
use pitpls_core::{
    common::Currency,
    crypto::{Action, CalculatedCrypto},
};

use crate::main_window::MainWindow;

pub type ChoiceState = SelectState<SearchableVec<&'static str>>;

const ACTIONS: [&str; 2] = ["Buy", "Sell"];
const CURRENCIES: [&str; 34] = [
    "PLN", "USD", "EUR", "GBP", "CHF", "AUD", "CAD", "NZD", "SGD", "HKD", "THB", "HUF", "UAH",
    "JPY", "CZK", "DKK", "ISK", "NOK", "SEK", "RON", "BGN", "TRY", "ILS", "CLP", "PHP", "MXN",
    "ZAR", "BRL", "MYR", "IDR", "INR", "KRW", "CNY", "XDR",
];

pub struct CryptoForm {
    pub editing_id: Option<String>,
    pub id: Entity<InputState>,
    pub date: Entity<InputState>,
    pub action: Entity<ChoiceState>,
    pub value: Entity<InputState>,
    pub value_currency: Entity<ChoiceState>,
    pub fee: Entity<InputState>,
    pub fee_currency: Entity<ChoiceState>,
    pub provider: Entity<InputState>,
}

impl CryptoForm {
    pub fn new(
        record: Option<&CalculatedCrypto>,
        window: &mut Window,
        cx: &mut Context<MainWindow>,
    ) -> Self {
        let editing_id = record.map(|record| record.id.clone());
        let id_value = record.map_or_else(String::new, |record| record.id.clone());
        let date_value = record.map_or_else(
            || Local::now().date_naive().format("%Y-%m-%d").to_string(),
            |record| record.date.format("%Y-%m-%d").to_string(),
        );
        let action_index = record.map_or(0, |record| match record.action {
            Action::FiatBuy => 0,
            Action::FiatSell => 1,
        });
        let value = record.map_or_else(String::new, |record| record.value.value.to_string());
        let value_currency = record.map_or(Currency::USD, |record| record.value.currency);
        let fee = record.map_or_else(String::new, |record| record.fee.value.to_string());
        let fee_currency = record.map_or(Currency::USD, |record| record.fee.currency);
        let provider = record.map_or_else(String::new, |record| record.provider.clone());

        Self {
            editing_id,
            id: input(cx, window, id_value, "Leave blank to auto-generate"),
            date: input(cx, window, date_value, "YYYY-MM-DD"),
            action: choice(cx, window, &ACTIONS, action_index, false),
            value: input(cx, window, value, "0.00"),
            value_currency: choice(
                cx,
                window,
                &CURRENCIES,
                currency_index(value_currency),
                true,
            ),
            fee: input(cx, window, fee, "0.00"),
            fee_currency: choice(cx, window, &CURRENCIES, currency_index(fee_currency), true),
            provider: input(cx, window, provider, "Provider"),
        }
    }

    pub fn create_input(&self, cx: &App) -> Result<CreateCryptoInput, String> {
        let id = value(&self.id, cx);
        Ok(CreateCryptoInput {
            id: (!id.trim().is_empty()).then(|| id.trim().to_string()),
            date: value(&self.date, cx),
            action: action(&self.action, cx)?,
            value: value(&self.value, cx).trim().to_string(),
            value_currency: currency(&self.value_currency, cx)?,
            fee: value(&self.fee, cx).trim().to_string(),
            fee_currency: currency(&self.fee_currency, cx)?,
            provider: value(&self.provider, cx).trim().to_string(),
        })
    }

    pub fn update_input(&self, cx: &App) -> Result<UpdateCryptoInput, String> {
        Ok(UpdateCryptoInput {
            id: self
                .editing_id
                .clone()
                .ok_or_else(|| "missing crypto ID".to_string())?,
            date: value(&self.date, cx),
            action: action(&self.action, cx)?,
            value: value(&self.value, cx).trim().to_string(),
            value_currency: currency(&self.value_currency, cx)?,
            fee: value(&self.fee, cx).trim().to_string(),
            fee_currency: currency(&self.fee_currency, cx)?,
            provider: value(&self.provider, cx).trim().to_string(),
        })
    }
}

fn input(
    cx: &mut Context<MainWindow>,
    window: &mut Window,
    default_value: String,
    placeholder: &'static str,
) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx)
            .default_value(default_value)
            .placeholder(placeholder)
    })
}

fn choice(
    cx: &mut Context<MainWindow>,
    window: &mut Window,
    choices: &[&'static str],
    selected: usize,
    searchable: bool,
) -> Entity<ChoiceState> {
    let choices = SearchableVec::new(choices.to_vec());
    cx.new(|cx| {
        SelectState::new(
            choices,
            Some(IndexPath::default().row(selected)),
            window,
            cx,
        )
        .searchable(searchable)
    })
}

fn value(input: &Entity<InputState>, cx: &App) -> String {
    input.read(cx).value().to_string()
}

fn selected(choice: &Entity<ChoiceState>, cx: &App) -> Result<&'static str, String> {
    choice
        .read(cx)
        .selected_value()
        .copied()
        .ok_or_else(|| "select a value".to_string())
}

fn action(choice: &Entity<ChoiceState>, cx: &App) -> Result<Action, String> {
    match selected(choice, cx)? {
        "Buy" => Ok(Action::FiatBuy),
        "Sell" => Ok(Action::FiatSell),
        value => Err(format!("unknown crypto action: {value}")),
    }
}

fn currency(choice: &Entity<ChoiceState>, cx: &App) -> Result<Currency, String> {
    selected(choice, cx)?.parse()
}

fn currency_index(currency: Currency) -> usize {
    CURRENCIES
        .iter()
        .position(|candidate| *candidate == currency.as_str())
        .unwrap_or(1)
}
