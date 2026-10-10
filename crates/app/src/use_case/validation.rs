use chrono::NaiveDate;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmountField {
    Value,
    Fee,
    TaxPaid,
}

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    InvalidYear {
        year: i32,
        min: i32,
        max: i32,
    },
    InvalidDate {
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    InvalidAmount {
        field: AmountField,
        value: String,
        #[source]
        source: rust_decimal::Error,
    },
    MissingId,
}

pub fn parse_date(value: &str) -> Result<NaiveDate, Error> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|source| Error::InvalidDate {
        value: value.to_owned(),
        source,
    })
}

pub fn parse_amount(value: &str, field: AmountField) -> Result<rust_decimal::Decimal, Error> {
    value.parse().map_err(|source| Error::InvalidAmount {
        field,
        value: value.to_owned(),
        source,
    })
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
