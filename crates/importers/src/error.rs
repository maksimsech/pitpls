use thiserror::Error;

pub type Result<T> = std::result::Result<T, ImportError>;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("Failed to read import file: {0}")]
    Read(#[from] std::io::Error),
    #[error("Failed to extract PDF text: {0}")]
    PdfExtract(#[from] pdf_extract::OutputError),
    #[error("Unexpected import format: {0}")]
    UnexpectedFormat(String),
    #[error("Missing header row")]
    MissingHeader,
    #[error("Unexpected header: {0}")]
    UnexpectedHeader(String),
    #[error("Missing column: {0}")]
    MissingColumn(String),
    #[error("Missing section: {0}")]
    MissingSection(String),
    #[error("Missing field `{name}` in {context}")]
    MissingField { name: String, context: String },
    #[error("Invalid field `{name}` in {context}: {value}")]
    InvalidField {
        name: String,
        value: String,
        context: String,
    },
    #[error("Data mismatch in {context}: expected {expected}, got {actual}")]
    DataMismatch {
        context: String,
        expected: String,
        actual: String,
    },
    #[error("Malformed row: expected at least {expected} fields, got {actual}: {row}")]
    MalformedRow {
        expected: usize,
        actual: usize,
        row: String,
    },
    #[error("Invalid timestamp `{value}`: {source}")]
    InvalidTimestamp {
        value: String,
        #[source]
        source: chrono::ParseError,
    },
    #[error("Invalid decimal `{value}`: {source}")]
    InvalidDecimal {
        value: String,
        #[source]
        source: rust_decimal::Error,
    },
    #[error("Invalid currency `{value}`: {message}")]
    InvalidCurrency { value: String, message: String },
    #[error("Invalid ISIN `{isin}`: {message}")]
    InvalidIsin { isin: String, message: String },
    #[error("{0}")]
    Other(String),
}

impl ImportError {
    pub fn unexpected_format(message: impl Into<String>) -> Self {
        Self::UnexpectedFormat(message.into())
    }

    pub fn missing_section(name: impl Into<String>) -> Self {
        Self::MissingSection(name.into())
    }

    pub fn missing_field(name: impl Into<String>, context: impl Into<String>) -> Self {
        Self::MissingField {
            name: name.into(),
            context: context.into(),
        }
    }

    pub fn invalid_field(
        name: impl Into<String>,
        value: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::InvalidField {
            name: name.into(),
            value: value.into(),
            context: context.into(),
        }
    }

    pub fn data_mismatch(
        context: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::DataMismatch {
            context: context.into(),
            expected: expected.into(),
            actual: actual.into(),
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

    pub fn invalid_currency(value: impl Into<String>, source: impl Into<String>) -> Self {
        Self::InvalidCurrency {
            value: value.into(),
            message: source.into(),
        }
    }

    pub fn invalid_isin(isin: impl Into<String>, source: impl Into<String>) -> Self {
        Self::InvalidIsin {
            isin: isin.into(),
            message: source.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}
