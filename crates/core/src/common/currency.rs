use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{self:?}")]
pub enum CurrencyParseError {
    UnknownCurrency { value: String },
}

macro_rules! currencies {
    ($($currency:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
        pub enum Currency {
            $($currency),+
        }

        impl Currency {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$currency => stringify!($currency)),+
                }
            }
        }

        impl Display for Currency {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str((*self).as_str())
            }
        }

        impl FromStr for Currency {
            type Err = CurrencyParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let normalized = s.trim();

                $(
                    if normalized.eq_ignore_ascii_case(stringify!($currency)) {
                        return Ok(Self::$currency);
                    }
                )+

                Err(CurrencyParseError::UnknownCurrency { value: s.to_owned() })
            }
        }
    };
}

currencies! {
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
