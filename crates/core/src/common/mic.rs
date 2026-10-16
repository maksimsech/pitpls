use std::{fmt::Display, str::FromStr};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{self:?}")]
pub enum MicParseError {
    InvalidCode { value: String },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Mic([u8; 4]);

impl Mic {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("MIC is valid ASCII")
    }
}

impl Display for Mic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Mic {
    type Err = MicParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes: [u8; 4] = s
            .trim()
            .as_bytes()
            .try_into()
            .ok()
            .filter(|bytes: &[u8; 4]| bytes.iter().all(u8::is_ascii_alphanumeric))
            .ok_or_else(|| MicParseError::InvalidCode {
                value: s.to_owned(),
            })?;

        Ok(Self(bytes.map(|byte| byte.to_ascii_uppercase())))
    }
}
