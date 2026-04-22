use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Copy, Serialize, Deserialize, Type)]
pub enum Country {
    Japan,
    USA,
}

impl std::fmt::Display for Country {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Country::Japan => "japan",
            Country::USA => "usa",
        };
        write!(f, "{str}")
    }
}

impl FromStr for Country {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "japan" => Country::Japan,
            "usa" => Country::USA,
            _ => {
                return Err(format!("Unknown country: {s}"));
            }
        })
    }
}

#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize, Type)]
pub enum Currency {
    EUR,
    USD,
    PLN,
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Currency::EUR => "EUR",
            Currency::USD => "USD",
            Currency::PLN => "PLN",
        };
        write!(f, "{str}")
    }
}

impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "eur" => Currency::EUR,
            "usd" => Currency::USD,
            "pln" => Currency::PLN,
            _ => {
                return Err(format!("Unknown currency: {s}"));
            }
        })
    }
}

#[derive(Clone, Copy, Type, Deserialize, Serialize)]
pub struct Amount {
    pub value: Decimal,
    pub currency: Currency,
}
