use chrono::NaiveDate;
use pitpls_core::common::{Country, CountryParseError, Currency, CurrencyParseError};
use rust_decimal::Decimal;
use thiserror::Error;

use crate::ImporterKind;

pub type Result<T> = std::result::Result<T, ImportError>;

#[derive(Clone, Copy, Debug)]
pub enum ImportField {
    Line,
    TotalRow,
    Ticker,
    Isin,
    TrailingTokens,
    GrossAmount,
    WithholdingTax,
    NetAmount,
    LocalCurrencyRate,
    SourceCurrency,
    TotalCurrency,
}

#[derive(Clone, Debug)]
pub enum ImportContext {
    RevolutOtherIncome,
    RevolutRow {
        ticker: Option<String>,
        date: NaiveDate,
    },
    RevolutTotal,
    T212Dividend {
        ticker: String,
        date: NaiveDate,
    },
}

#[derive(Debug)]
pub struct ImportAmounts {
    pub gross: Decimal,
    pub tax: Decimal,
    pub net: Decimal,
}

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum ImportError {
    Read(#[from] std::io::Error),
    PdfExtract(#[from] pdf_extract::OutputError),
    UnexpectedFormat {
        expected: ImporterKind,
    },
    MissingHeader,
    UnexpectedHeader(String),
    /// The literal column name required by the source file format.
    MissingColumn(String),
    MissingSection(ImportContext),
    MissingField {
        field: ImportField,
        context: ImportContext,
    },
    InvalidField {
        field: ImportField,
        value: String,
        context: ImportContext,
    },
    CurrencyMismatch {
        field: ImportField,
        context: ImportContext,
        expected: Currency,
        actual: Currency,
    },
    CountryMismatch {
        context: ImportContext,
        isin: String,
        expected: Country,
        actual: String,
    },
    AmountMismatch {
        context: ImportContext,
        amounts: ImportAmounts,
    },
    TotalsMismatch {
        currency: Currency,
        expected: ImportAmounts,
        actual: ImportAmounts,
    },
    MalformedRow {
        expected: usize,
        actual: usize,
        row: String,
    },
    InvalidTimestamp {
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    InvalidDecimal {
        value: String,
        #[source]
        source: rust_decimal::Error,
    },
    InvalidCurrency {
        value: String,
        #[source]
        source: CurrencyParseError,
    },
    InvalidIsin {
        isin: String,
        #[source]
        source: CountryParseError,
    },
}

impl ImportError {
    pub fn missing_field(field: ImportField, context: ImportContext) -> Self {
        Self::MissingField { field, context }
    }

    pub fn invalid_field(
        field: ImportField,
        value: impl Into<String>,
        context: ImportContext,
    ) -> Self {
        Self::InvalidField {
            field,
            value: value.into(),
            context,
        }
    }

    pub fn malformed_row(expected: usize, actual: usize, row: impl Into<String>) -> Self {
        Self::MalformedRow {
            expected,
            actual,
            row: row.into(),
        }
    }

    pub fn invalid_timestamp(value: impl Into<String>, source: chrono::ParseError) -> Self {
        Self::InvalidTimestamp {
            value: value.into(),
            source,
        }
    }

    pub fn invalid_decimal(value: impl Into<String>, source: rust_decimal::Error) -> Self {
        Self::InvalidDecimal {
            value: value.into(),
            source,
        }
    }

    pub fn invalid_currency(value: impl Into<String>, source: CurrencyParseError) -> Self {
        Self::InvalidCurrency {
            value: value.into(),
            source,
        }
    }

    pub fn invalid_isin(isin: impl Into<String>, source: CountryParseError) -> Self {
        Self::InvalidIsin {
            isin: isin.into(),
            source,
        }
    }
}
