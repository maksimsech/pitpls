use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize, Type)]
pub enum Currency {
    EUR,
    USD,
    PLN,
}

impl Display for Currency {
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
