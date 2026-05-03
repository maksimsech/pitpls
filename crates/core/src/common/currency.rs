use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize, Type)]
pub enum Currency {
    THB,
    USD,
    AUD,
    HKD,
    CAD,
    NZD,
    SGD,
    EUR,
    HUF,
    CHF,
    GBP,
    UAH,
    JPY,
    CZK,
    DKK,
    ISK,
    NOK,
    SEK,
    RON,
    BGN,
    TRY,
    ILS,
    CLP,
    PHP,
    MXN,
    ZAR,
    BRL,
    MYR,
    IDR,
    INR,
    KRW,
    CNY,
    XDR,
    PLN,
}

impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Currency::THB => "THB",
            Currency::USD => "USD",
            Currency::AUD => "AUD",
            Currency::HKD => "HKD",
            Currency::CAD => "CAD",
            Currency::NZD => "NZD",
            Currency::SGD => "SGD",
            Currency::EUR => "EUR",
            Currency::HUF => "HUF",
            Currency::CHF => "CHF",
            Currency::GBP => "GBP",
            Currency::UAH => "UAH",
            Currency::JPY => "JPY",
            Currency::CZK => "CZK",
            Currency::DKK => "DKK",
            Currency::ISK => "ISK",
            Currency::NOK => "NOK",
            Currency::SEK => "SEK",
            Currency::RON => "RON",
            Currency::BGN => "BGN",
            Currency::TRY => "TRY",
            Currency::ILS => "ILS",
            Currency::CLP => "CLP",
            Currency::PHP => "PHP",
            Currency::MXN => "MXN",
            Currency::ZAR => "ZAR",
            Currency::BRL => "BRL",
            Currency::MYR => "MYR",
            Currency::IDR => "IDR",
            Currency::INR => "INR",
            Currency::KRW => "KRW",
            Currency::CNY => "CNY",
            Currency::XDR => "XDR",
            Currency::PLN => "PLN",
        };
        write!(f, "{str}")
    }
}

impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_uppercase();

        Ok(match normalized.as_str() {
            "THB" => Currency::THB,
            "USD" => Currency::USD,
            "AUD" => Currency::AUD,
            "HKD" => Currency::HKD,
            "CAD" => Currency::CAD,
            "NZD" => Currency::NZD,
            "SGD" => Currency::SGD,
            "EUR" => Currency::EUR,
            "HUF" => Currency::HUF,
            "CHF" => Currency::CHF,
            "GBP" => Currency::GBP,
            "UAH" => Currency::UAH,
            "JPY" => Currency::JPY,
            "CZK" => Currency::CZK,
            "DKK" => Currency::DKK,
            "ISK" => Currency::ISK,
            "NOK" => Currency::NOK,
            "SEK" => Currency::SEK,
            "RON" => Currency::RON,
            "BGN" => Currency::BGN,
            "TRY" => Currency::TRY,
            "ILS" => Currency::ILS,
            "CLP" => Currency::CLP,
            "PHP" => Currency::PHP,
            "MXN" => Currency::MXN,
            "ZAR" => Currency::ZAR,
            "BRL" => Currency::BRL,
            "MYR" => Currency::MYR,
            "IDR" => Currency::IDR,
            "INR" => Currency::INR,
            "KRW" => Currency::KRW,
            "CNY" => Currency::CNY,
            "XDR" => Currency::XDR,
            "PLN" => Currency::PLN,
            _ => {
                return Err(format!("Unknown currency: {s}"));
            }
        })
    }
}
