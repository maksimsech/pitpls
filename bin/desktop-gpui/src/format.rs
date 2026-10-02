use pitpls_core::common::Amount;
use rust_decimal::Decimal;

pub fn money(value: Decimal) -> gpui_kit::SharedString {
    // Keep decimal precision throughout; only the displayed text is rounded.
    format!("{:.2}", value.round_dp(2)).into()
}

pub fn pln(value: Decimal) -> gpui_kit::SharedString {
    format!("{} PLN", money(value)).into()
}

pub fn exact_pln(value: Decimal) -> gpui_kit::SharedString {
    format!("{} PLN", value.normalize()).into()
}

pub fn amount(value: Amount) -> gpui_kit::SharedString {
    format!("{} {}", value.value.normalize(), value.currency).into()
}
