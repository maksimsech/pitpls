use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Country {
    Japan,
    USA,
    Other(IsinCountryCode),
}

impl Display for Country {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code().as_str())
    }
}

impl FromStr for Country {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IsinCountryCode::from_str(s).map(Self::from)
    }
}

impl From<IsinCountryCode> for Country {
    fn from(code: IsinCountryCode) -> Self {
        match code {
            IsinCountryCode::JP => Country::Japan,
            IsinCountryCode::US => Country::USA,
            _ => Country::Other(code),
        }
    }
}

impl Country {
    pub fn from_isin(isin: &str) -> Result<Self, String> {
        IsinCountryCode::from_isin(isin).map(Self::from)
    }

    pub fn code(&self) -> IsinCountryCode {
        match self {
            Country::Japan => IsinCountryCode::JP,
            Country::USA => IsinCountryCode::US,
            Country::Other(code) => *code,
        }
    }
}

impl Serialize for Country {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.code().as_str())
    }
}

impl<'de> Deserialize<'de> for Country {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let code = String::deserialize(deserializer)?;
        Country::from_str(&code).map_err(Error::custom)
    }
}

impl specta::Type for Country {
    fn inline(
        type_map: &mut specta::TypeCollection,
        generics: specta::Generics,
    ) -> specta::datatype::DataType {
        <String as specta::Type>::inline(type_map, generics)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IsinCountryCode([u8; 2]);

impl IsinCountryCode {
    pub const JP: Self = Self(*b"JP");
    pub const US: Self = Self(*b"US");

    pub fn from_isin(isin: &str) -> Result<Self, String> {
        let trimmed = isin.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() < 2 {
            return Err(format!("Invalid ISIN: {isin}"));
        }
        Self::from_bytes([bytes[0], bytes[1]])
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("ISIN country code is valid ASCII")
    }

    fn from_bytes(bytes: [u8; 2]) -> Result<Self, String> {
        if !bytes.iter().all(u8::is_ascii_alphabetic) {
            return Err(format!(
                "Invalid ISIN country code: {}{}",
                bytes[0] as char, bytes[1] as char
            ));
        }

        Ok(Self([
            bytes[0].to_ascii_uppercase(),
            bytes[1].to_ascii_uppercase(),
        ]))
    }
}

impl Display for IsinCountryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IsinCountryCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() != 2 {
            return Err(format!("Invalid ISIN country code: {s}"));
        }
        Self::from_bytes([bytes[0], bytes[1]])
    }
}
