use chrono::NaiveDate;
use gpui_kit::SharedString;
use pitpls_core::common::Amount;
use rust_decimal::Decimal;
use std::fmt::Display;

pub const DATE_FORMAT: &str = "%d.%m.%Y";

/// A narrow no-break space.
pub const THOUSANDS: char = '\u{202f}';
pub const EXTRA_DIGITS: usize = 3;

pub fn date(value: NaiveDate) -> DisplayText {
    DisplayText::plain(value.format(DATE_FORMAT).to_string())
}

#[derive(Clone, PartialEq)]
pub struct DisplayText {
    pub main: SharedString,
    pub extra: SharedString,
    pub more: bool,
    pub unit: Option<SharedString>,
    pub full: SharedString,
    /// What a copy button copies: for numbers, every digit in Polish format
    /// (comma decimal, no thousands separator, no unit).
    pub copy: SharedString,
}

impl DisplayText {
    pub fn plain(text: impl Into<SharedString>) -> Self {
        let text = text.into();
        Self {
            main: text.clone(),
            extra: SharedString::default(),
            more: false,
            unit: None,
            full: text.clone(),
            copy: text,
        }
    }

    fn with_unit(self, unit: impl Display) -> Self {
        Self {
            full: format!("{} {unit}", self.full).into(),
            unit: Some(unit.to_string().into()),
            ..self
        }
    }
}

/// Never rounds: digits past the second decimal are drawn faint, at most
/// `EXTRA_DIGITS` of them, and a "…" marks any beyond that. `full` and `copy`
/// keep every digit.
pub fn money(value: Decimal) -> DisplayText {
    let value = value.normalize();
    let digits = format!("{:.*}", value.scale().max(2) as usize, value.abs());
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits.as_str(), "00"));
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let grouped = format!("{sign}{}", group_thousands(whole));
    let (cents, rest) = fraction.split_at(2);
    let shown = rest.len().min(EXTRA_DIGITS);
    DisplayText {
        main: format!("{grouped}.{cents}").into(),
        extra: rest[..shown].to_owned().into(),
        more: rest.len() > shown,
        unit: None,
        full: format!("{grouped}.{fraction}").into(),
        copy: format!("{sign}{whole},{fraction}").into(),
    }
}

pub fn record_count(count: u32) -> String {
    match count {
        0 => "no records".to_owned(),
        1 => "1 record".to_owned(),
        count => format!("{count} records"),
    }
}

pub fn pln(value: Decimal) -> DisplayText {
    money(value).with_unit("PLN")
}

pub fn amount(value: Amount) -> DisplayText {
    money(value.value).with_unit(value.currency)
}

fn group_thousands(digits: &str) -> String {
    let mut grouped = String::with_capacity(digits.len() * 2);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(THOUSANDS);
        }
        grouped.push(digit);
    }
    grouped
}
