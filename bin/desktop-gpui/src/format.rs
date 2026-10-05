use chrono::NaiveDate;
use gpui_kit::SharedString;
use pitpls_core::common::Amount;
use rust_decimal::{Decimal, RoundingStrategy};
use std::fmt::Display;

pub const DATE_FORMAT: &str = "%d.%m.%Y";

pub fn date(value: NaiveDate) -> DisplayText {
    DisplayText::plain(value.format(DATE_FORMAT).to_string())
}

#[derive(Clone, PartialEq)]
pub struct DisplayText {
    pub text: SharedString,
    pub full: Option<SharedString>,
}

impl DisplayText {
    pub fn plain(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            full: None,
        }
    }

    fn suffixed(self, suffix: impl Display) -> Self {
        Self {
            text: format!("{} {suffix}", self.text).into(),
            full: self.full.map(|full| format!("{full} {suffix}").into()),
        }
    }
}

/// Money in any currency, with at least two decimals. Never rounds: digits past
/// the second decimal are cut off and marked with an ellipsis, and the full
/// value is kept for a tooltip.
pub fn money(value: Decimal) -> DisplayText {
    let value = value.normalize();
    let full = format!("{:.*}", value.scale().max(2) as usize, value);
    if value.scale() <= 2 {
        return DisplayText::plain(full);
    }
    let shown = value.round_dp_with_strategy(2, RoundingStrategy::ToZero);
    DisplayText {
        text: format!("{shown}…").into(),
        full: Some(full.into()),
    }
}

pub fn pln(value: Decimal) -> DisplayText {
    money(value).suffixed("PLN")
}

pub fn amount(value: Amount) -> DisplayText {
    money(value.value).suffixed(value.currency)
}
