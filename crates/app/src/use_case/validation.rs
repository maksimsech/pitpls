use chrono::{DateTime, NaiveDateTime, Utc};
use pitpls_core::time::from_local;
use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmountField {
    Value,
    Fee,
    TaxPaid,
    Shares,
    Price,
    Amount,
    SplitBefore,
    SplitAfter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextField {
    Ticker,
    Provider,
}

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    InvalidYear {
        year: i32,
        min: i32,
        max: i32,
    },
    InvalidTime {
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    NonexistentTime(NaiveDateTime),
    InvalidAmount {
        field: AmountField,
        value: String,
        #[source]
        source: rust_decimal::Error,
    },
    InvalidSplit {
        field: AmountField,
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },
    NotPositive {
        field: AmountField,
        value: Decimal,
    },
    Negative {
        field: AmountField,
        value: Decimal,
    },
    Empty(TextField),
    MissingId,
}

pub fn parse_time(value: &str) -> Result<DateTime<Utc>, Error> {
    let local = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").map_err(|source| {
        Error::InvalidTime {
            value: value.to_owned(),
            source,
        }
    })?;
    from_local(local).ok_or(Error::NonexistentTime(local))
}

pub fn parse_amount(value: &str, field: AmountField) -> Result<Decimal, Error> {
    value.parse().map_err(|source| Error::InvalidAmount {
        field,
        value: value.to_owned(),
        source,
    })
}

pub fn parse_split(value: &str, field: AmountField) -> Result<u32, Error> {
    value.trim().parse().map_err(|source| Error::InvalidSplit {
        field,
        value: value.to_owned(),
        source,
    })
}

pub fn validate_positive(value: Decimal, field: AmountField) -> Result<(), Error> {
    if value > Decimal::ZERO {
        Ok(())
    } else {
        Err(Error::NotPositive { field, value })
    }
}

pub fn validate_not_negative(value: Decimal, field: AmountField) -> Result<(), Error> {
    if value < Decimal::ZERO {
        Err(Error::Negative { field, value })
    } else {
        Ok(())
    }
}

pub fn validate_not_empty(value: &str, field: TextField) -> Result<(), Error> {
    if value.trim().is_empty() {
        Err(Error::Empty(field))
    } else {
        Ok(())
    }
}

pub fn validate_year(year: i32) -> Result<(), Error> {
    if (1900..=2100).contains(&year) {
        Ok(())
    } else {
        Err(Error::InvalidYear {
            year,
            min: 1900,
            max: 2100,
        })
    }
}

pub fn validate_optional_year(year: Option<i32>) -> Result<(), Error> {
    year.map_or(Ok(()), validate_year)
}
