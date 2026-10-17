use crate::i18n::plural;
use chrono::NaiveDate;
use gpui_kit::SharedString;
use pitpls_core::common::Amount;
use rust_decimal::Decimal;
use rust_i18n::t;
use std::fmt::Display;

pub const DATE_FORMAT: &str = "%d.%m.%Y";
pub const DATE_TIME_FORMAT: &str = "%d.%m.%Y %H:%M:%S";

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
    let (sign, shown_sign) = signs(value);
    let grouped = format!("{shown_sign}{}", group_thousands(whole));
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

pub fn shares(value: Decimal) -> DisplayText {
    let value = value.normalize();
    if !value.fract().is_zero() {
        return money(value);
    }
    let (sign, shown_sign) = signs(value);
    let digits = value.abs().to_string();
    let grouped = format!("{shown_sign}{}", group_thousands(&digits));
    DisplayText {
        main: grouped.clone().into(),
        extra: SharedString::default(),
        more: false,
        unit: None,
        full: grouped.into(),
        copy: format!("{sign}{digits}").into(),
    }
}

pub fn count(value: usize) -> DisplayText {
    DisplayText::plain(value.to_string())
}

pub fn record_count(count: u32) -> String {
    match count {
        0 => t!("records.none").into_owned(),
        count => plural("records.count", count.into()).into_owned(),
    }
}

pub fn pln(value: Decimal) -> DisplayText {
    money(value).with_unit("PLN")
}

pub fn amount(value: Amount) -> DisplayText {
    money(value.value).with_unit(value.currency)
}

// The typographic minus on screen; copies keep the ASCII one, which forms and spreadsheets parse.
fn signs(value: Decimal) -> (&'static str, &'static str) {
    if value.is_sign_negative() {
        ("-", "\u{2212}")
    } else {
        ("", "")
    }
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

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    fn parts(text: &DisplayText) -> (&str, &str, bool, &str) {
        (&text.main, &text.extra, text.more, &text.copy)
    }

    #[test]
    fn money_never_rounds() {
        let value = money(dec!(1234.56789));
        assert_eq!(
            parts(&value),
            ("1\u{202f}234.56", "789", false, "1234,56789")
        );
        assert_eq!(value.full.as_ref(), "1\u{202f}234.56789");

        let long = money(dec!(0.1234567));
        assert_eq!(parts(&long), ("0.12", "345", true, "0,1234567"));
        assert_eq!(long.full.as_ref(), "0.1234567");

        assert_eq!(parts(&money(dec!(1.5))), ("1.50", "", false, "1,50"));
    }

    #[test]
    fn minus_is_typographic_on_screen_and_ascii_when_copied() {
        assert_eq!(
            parts(&money(dec!(-1500.5))),
            ("\u{2212}1\u{202f}500.50", "", false, "-1500,50")
        );
    }

    #[test]
    fn units_follow_the_value() {
        assert_eq!(pln(dec!(2)).full.as_ref(), "2.00 PLN");
        let usd = amount(Amount {
            value: dec!(0.125),
            currency: pitpls_core::common::Currency::USD,
        });
        assert_eq!(
            (usd.main.as_ref(), usd.extra.as_ref(), usd.full.as_ref()),
            ("0.12", "5", "0.125 USD")
        );
    }

    #[test]
    fn whole_shares_have_no_decimals() {
        assert_eq!(
            parts(&shares(dec!(1000))),
            ("1\u{202f}000", "", false, "1000")
        );
        assert_eq!(parts(&shares(dec!(0.5))), ("0.50", "", false, "0,50"));
    }
}
