use gpui_kit::SharedString;
use pitpls_app::use_case::{
    AmountField, ValidationError,
    crypto::Error as CryptoError,
    dividend::Error as DividendError,
    import::Error as StatementImportError,
    interest::Error as InterestError,
    rate::{ApiImportError as RateApiError, CsvImportError as RateCsvError},
    tax::Error as TaxError,
    year::Error as YearError,
};
use pitpls_core::{
    common::CountryParseError, crypto::CalculateSellBuyValuesError,
    dividend::CalculateDividendTaxError, interest::CalculateInterestTaxError,
    rate::RateConverterError, summary::CalculateTaxSummaryError,
};
use pitpls_db::{OpenDatabaseError, RepositoryError};
use pitpls_importers::{ImportAmounts, ImportContext, ImportError, ImportField, ImporterKind};
use pitpls_nbr::{ApiImportError, CsvImportError};

use crate::{format::date, services::Error as ServiceError};

pub fn importer_name(kind: ImporterKind) -> &'static str {
    match kind {
        ImporterKind::T212 => "Trading 212",
        ImporterKind::Revolut => "Revolut",
        ImporterKind::Coinbase => "Coinbase",
    }
}

impl From<ServiceError> for SharedString {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::Crypto(error) => crypto_use_case_error(&error),
            ServiceError::Dividend(error) => dividend_use_case_error(&error),
            ServiceError::Interest(error) => interest_use_case_error(&error),
            ServiceError::Import(error) => match error {
                StatementImportError::Parse(error) => import_error(&error),
                StatementImportError::Repository(error) => repository_error(&error),
            },
            ServiceError::RateCsv(error) => match error {
                RateCsvError::Parse(error) => csv_error(&error),
                RateCsvError::Repository(error) => repository_error(&error),
            },
            ServiceError::RateApi(error) => match error {
                RateApiError::Validation(error) => validation_error(&error),
                RateApiError::Fetch(error) => api_error(&error),
                RateApiError::Repository(error) => repository_error(&error),
            },
            ServiceError::Tax(error) => match error {
                TaxError::Validation(error) => validation_error(&error),
                TaxError::Repository(error) => repository_error(&error),
                TaxError::Calculation(error) => match error {
                    CalculateTaxSummaryError::Crypto(error) => crypto_error(&error),
                    CalculateTaxSummaryError::Dividend(error) => dividend_error(&error),
                    CalculateTaxSummaryError::Interest(error) => interest_error(&error),
                },
            },
            ServiceError::Year(error) => match error {
                YearError::Validation(error) => validation_error(&error),
                YearError::Repository(error) => repository_error(&error),
            },
            ServiceError::Repository(error) => repository_error(&error),
            ServiceError::OpenDatabase(error) => match error {
                OpenDatabaseError::CreateDirectory(source) => {
                    format!("Failed to create the database directory: {source}")
                }
                OpenDatabaseError::Connect(source) => {
                    format!("Failed to open the database: {source}")
                }
                OpenDatabaseError::Migrate(source) => {
                    format!("Failed to migrate the database: {source}")
                }
            },
            ServiceError::Background(source) => format!("Background operation failed: {source}"),
            ServiceError::ReadPreferences(source) => {
                format!("Could not read preferences: {source}")
            }
            ServiceError::ParsePreferences(source) => {
                format!("Could not read preferences: {source}")
            }
            ServiceError::WritePreferences(source) => {
                format!("Could not save preferences: {source}")
            }
            ServiceError::SerializePreferences(source) => {
                format!("Could not save preferences: {source}")
            }
        }
        .into()
    }
}

fn validation_error(error: &ValidationError) -> String {
    match error {
        ValidationError::InvalidYear { year, min, max } => {
            format!("Year {year} out of range ({min}–{max})")
        }
        ValidationError::InvalidDate { value, .. } => {
            format!("Invalid date '{value}' (expected YYYY-MM-DD)")
        }
        ValidationError::InvalidAmount { field, value, .. } => {
            let field = match field {
                AmountField::Value => "value",
                AmountField::Fee => "fee",
                AmountField::TaxPaid => "tax paid",
            };
            format!("Invalid {field}: {value}")
        }
        ValidationError::MissingId => "ID is required".into(),
    }
}

fn crypto_use_case_error(error: &CryptoError) -> String {
    match error {
        CryptoError::Validation(error) => validation_error(error),
        CryptoError::Repository(error) => repository_error(error),
        CryptoError::DuplicateId(id) => format!("Crypto with ID '{id}' already exists"),
        CryptoError::NotFound(id) => format!("Crypto with ID '{id}' not found"),
        CryptoError::Calculation(error) => crypto_error(error),
        CryptoError::NothingToPreview => "Nothing to preview".into(),
    }
}

fn dividend_use_case_error(error: &DividendError) -> String {
    match error {
        DividendError::Validation(error) => validation_error(error),
        DividendError::Repository(error) => repository_error(error),
        DividendError::DuplicateId(id) => format!("Dividend with ID '{id}' already exists"),
        DividendError::NotFound(id) => format!("Dividend with ID '{id}' not found"),
        DividendError::Calculation(error) => dividend_error(error),
        DividendError::NothingToPreview => "Nothing to preview".into(),
    }
}

fn interest_use_case_error(error: &InterestError) -> String {
    match error {
        InterestError::Validation(error) => validation_error(error),
        InterestError::Repository(error) => repository_error(error),
        InterestError::DuplicateId(id) => format!("Interest with ID '{id}' already exists"),
        InterestError::NotFound(id) => format!("Interest with ID '{id}' not found"),
        InterestError::Calculation(error) => interest_error(error),
        InterestError::NothingToPreview => "Nothing to preview".into(),
    }
}

fn repository_error(error: &RepositoryError) -> String {
    match error {
        RepositoryError::Database(source) => format!("Database error: {source}"),
        RepositoryError::Serialization(_) => "A stored value has an invalid format.".into(),
        RepositoryError::Decimal(_) => "A stored amount has an invalid format.".into(),
        RepositoryError::InvalidYear(year) => format!("Year {year} is out of range"),
    }
}

fn crypto_error(error: &CalculateSellBuyValuesError) -> String {
    match error {
        CalculateSellBuyValuesError::ValueConversion(source) => format!(
            "Failed to convert crypto value to PLN: {}",
            rate_error(source)
        ),
        CalculateSellBuyValuesError::FeeConversion(source) => format!(
            "Failed to convert crypto fee to PLN: {}",
            rate_error(source)
        ),
    }
}

fn dividend_error(error: &CalculateDividendTaxError) -> String {
    match error {
        CalculateDividendTaxError::DividendConversion(source) => format!(
            "Failed to convert dividend value to PLN: {}",
            rate_error(source)
        ),
        CalculateDividendTaxError::PaidTaxConversion(source) => format!(
            "Failed to convert paid dividend tax to PLN: {}",
            rate_error(source)
        ),
    }
}

fn interest_error(error: &CalculateInterestTaxError) -> String {
    match error {
        CalculateInterestTaxError::InterestConversion(source) => format!(
            "Failed to convert interest value to PLN: {}",
            rate_error(source)
        ),
    }
}

fn rate_error(error: &RateConverterError) -> String {
    match error {
        RateConverterError::NoRatesAvailable => "No exchange rates are available.".into(),
        RateConverterError::StepLimitReached {
            steps,
            currency,
            date: at,
        } => format!(
            "No {currency} rate was found in the {steps} days before {}.",
            date(*at).main
        ),
    }
}

fn api_error(error: &ApiImportError) -> String {
    match error {
        ApiImportError::YearTooEarly { min_year } => {
            format!("NBP rates are available from {min_year}")
        }
        ApiImportError::FutureYear => "Cannot import NBP rates for a future year".into(),
        ApiImportError::InvalidDate => "Invalid date".into(),
        ApiImportError::InvalidDateRange => "Invalid NBP date range".into(),
        ApiImportError::UnsupportedCurrency { currency } => {
            format!("{currency} rates are not imported from NBP")
        }
        ApiImportError::Request { source } => format!("Failed to request NBP rates: {source}"),
        ApiImportError::HttpStatus {
            status,
            code,
            start_date,
            end_date,
        } => format!(
            "NBP API returned {} for {code} rates from {} to {}",
            status.as_u16(),
            date(*start_date).main,
            date(*end_date).main
        ),
        ApiImportError::Response { source } => format!("Failed to parse NBP response: {source}"),
    }
}

fn csv_error(error: &CsvImportError) -> String {
    match error {
        CsvImportError::InvalidExtension => "Select a .csv file.".into(),
        CsvImportError::Empty => "The rates CSV is empty.".into(),
        CsvImportError::InvalidFormat => "Invalid rates CSV format".into(),
        CsvImportError::Read(source) => format!("Failed to read rates CSV: {source}"),
        CsvImportError::InvalidDate { line, .. } => format!("Invalid date at line {line}"),
        CsvImportError::InvalidUnit { header, .. } => {
            format!("Invalid rate unit in header '{header}'")
        }
        CsvImportError::InvalidRate { line, column, .. } => {
            format!("Invalid rate in column '{column}' at line {line}")
        }
        CsvImportError::DuplicateRate {
            currency,
            date: at,
            line,
        } => format!(
            "Duplicate rate for {currency} on {} at line {line}",
            date(*at).main
        ),
    }
}

fn import_error(error: &ImportError) -> String {
    match error {
        ImportError::Read(source) => format!("Failed to read import file: {source}"),
        ImportError::PdfExtract(source) => format!("Failed to extract PDF text: {source}"),
        ImportError::UnexpectedFormat { expected } => match expected {
            ImporterKind::Revolut => "Not a Revolut Profit and Loss Statement PDF".into(),
            ImporterKind::T212 | ImporterKind::Coinbase => {
                format!("Unexpected format for {}", importer_name(*expected))
            }
        },
        ImportError::MissingHeader => "Missing header row".into(),
        ImportError::UnexpectedHeader(header) => format!("Unexpected header: {header}"),
        ImportError::MissingColumn(column) => format!("Missing column: {column}"),
        ImportError::MissingSection(context) => {
            format!("Missing section: {}", import_context(context))
        }
        ImportError::MissingField { field, context } => format!(
            "Missing field '{}' in {}",
            import_field(*field),
            import_context(context)
        ),
        ImportError::InvalidField {
            field,
            value,
            context,
        } => format!(
            "Invalid field '{}' in {}: {value}",
            import_field(*field),
            import_context(context)
        ),
        ImportError::CurrencyMismatch {
            field,
            context,
            expected,
            actual,
        } => format!(
            "Currency mismatch for {} in {}: expected {expected}, got {actual}",
            import_field(*field),
            import_context(context)
        ),
        ImportError::CountryMismatch {
            context,
            isin,
            expected,
            actual,
        } => format!(
            "Country mismatch in {}: ISIN {isin} maps to {expected}, row has {actual}",
            import_context(context)
        ),
        ImportError::AmountMismatch { context, amounts } => format!(
            "Amount mismatch in {}: {} − {} does not equal {}",
            import_context(context),
            amounts.gross,
            amounts.tax,
            amounts.net
        ),
        ImportError::TotalsMismatch {
            currency,
            expected,
            actual,
        } => format!(
            "Total mismatch in Revolut Other income & fees table ({currency}): rows {}, total {}",
            import_amounts(expected),
            import_amounts(actual)
        ),
        ImportError::MalformedRow {
            expected,
            actual,
            row,
        } => format!("Malformed row: expected at least {expected} fields, got {actual}: {row}"),
        ImportError::InvalidTimestamp { value, .. } => format!("Invalid timestamp '{value}'"),
        ImportError::InvalidDecimal { value, .. } => format!("Invalid decimal '{value}'"),
        ImportError::InvalidCurrency { value, .. } => format!("Invalid currency '{value}'"),
        ImportError::InvalidIsin { isin, .. } => format!("Invalid ISIN '{isin}'"),
    }
}

fn import_field(field: ImportField) -> &'static str {
    match field {
        ImportField::Line => "line",
        ImportField::TotalRow => "Total row",
        ImportField::Ticker => "ticker",
        ImportField::Isin => "ISIN",
        ImportField::TrailingTokens => "trailing tokens",
        ImportField::GrossAmount => "gross amount",
        ImportField::WithholdingTax => "withholding tax",
        ImportField::NetAmount => "net amount",
        ImportField::LocalCurrencyRate => "local currency rate",
        ImportField::SourceCurrency => "source currency",
        ImportField::TotalCurrency => "total currency",
    }
}

fn import_context(context: &ImportContext) -> String {
    match context {
        ImportContext::RevolutOtherIncome => "Revolut Other income & fees table".into(),
        ImportContext::RevolutRow { ticker, date: at } => match ticker {
            Some(ticker) => format!("Revolut row {ticker} on {}", date(*at).main),
            None => format!("Revolut dividend on {}", date(*at).main),
        },
        ImportContext::RevolutTotal => "Revolut total".into(),
        ImportContext::T212Dividend { ticker, date: at } => {
            format!("Trading 212 dividend {ticker} on {}", date(*at).main)
        }
    }
}

fn import_amounts(amounts: &ImportAmounts) -> String {
    format!(
        "gross {}, tax {}, net {}",
        amounts.gross, amounts.tax, amounts.net
    )
}

pub fn country_error(error: CountryParseError) -> String {
    match error {
        CountryParseError::InvalidIsin { value } => format!("Invalid ISIN: {value}"),
        CountryParseError::InvalidCode { value } => {
            format!("Invalid country code '{value}': enter two letters, such as US or JP")
        }
    }
}
