use chrono::NaiveDate;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CsvImportError {
    #[error("Invalid extension")]
    InvalidExtension,
    #[error("Empty rates CSV")]
    Empty,
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Failed to read rates CSV: {0}")]
    Read(#[from] std::io::Error),
    #[error("Invalid date at line {line}: {source}")]
    InvalidDate {
        line: usize,
        #[source]
        source: chrono::ParseError,
    },
    #[error("Invalid rate unit in header `{header}`: {source}")]
    InvalidUnit {
        header: String,
        #[source]
        source: rust_decimal::Error,
    },
    #[error("Invalid rate in column `{column}` at line {line}: {source}")]
    InvalidRate {
        line: usize,
        column: String,
        #[source]
        source: rust_decimal::Error,
    },
    #[error("Duplicate rate for {currency} on {date} at line {line}")]
    DuplicateRate {
        currency: String,
        date: NaiveDate,
        line: usize,
    },
}

#[derive(Debug, Error)]
pub enum ApiImportError {
    #[error("NBP rates are available from {min_year}")]
    YearTooEarly { min_year: i32 },
    #[error("Cannot import NBP rates for a future year")]
    FutureYear,
    #[error("Invalid date")]
    InvalidDate,
    #[error("Invalid NBP date range")]
    InvalidDateRange,
    #[error("{currency} rates are not imported from NBP")]
    UnsupportedCurrency { currency: String },
    #[error("Failed to request NBP rates: {source}")]
    Request {
        #[source]
        source: reqwest::Error,
    },
    #[error("NBP API returned {status} for {code} rates from {start_date} to {end_date}")]
    HttpStatus {
        status: StatusCode,
        code: &'static str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
    #[error("Failed to parse NBP response: {source}")]
    Response {
        #[source]
        source: reqwest::Error,
    },
}
