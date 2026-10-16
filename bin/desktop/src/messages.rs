use gpui_kit::SharedString;
use pitpls_app::use_case::{
    AmountField, TextField, ValidationError,
    crypto::Error as CryptoError,
    dividend::Error as DividendError,
    exchange::RefreshError as ExchangeRefreshError,
    import::Error as StatementImportError,
    interest::Error as InterestError,
    portfolio::Error as PortfolioError,
    rate::{ApiImportError as RateApiError, CsvImportError as RateCsvError},
    stock::Error as StockError,
    tax::Error as TaxError,
    year::Error as YearError,
};
use pitpls_core::{
    common::CountryParseError, crypto::CalculateSellBuyValuesError,
    dividend::CalculateDividendTaxError, interest::CalculateInterestTaxError,
    rate::RateConverterError, stock::CalculateStocksError, summary::CalculateTaxSummaryError,
};
use pitpls_db::{OpenDatabaseError, RepositoryError};
use pitpls_importers::{ImportAmounts, ImportContext, ImportError, ImportField, ImporterKind};
use pitpls_mic::{DownloadError, ParseError as MicParseError};
use pitpls_nbr::{ApiImportError, CsvImportError};

use crate::{
    format::{DATE_TIME_FORMAT, date},
    services::Error as ServiceError,
};
use rust_i18n::t;
use std::borrow::Cow;

pub fn importer_name(kind: ImporterKind) -> &'static str {
    match kind {
        ImporterKind::T212 => "Trading 212",
        ImporterKind::Revolut => "Revolut",
        ImporterKind::Coinbase => "Coinbase",
    }
}

impl From<ServiceError> for SharedString {
    fn from(error: ServiceError) -> Self {
        (&error).into()
    }
}

impl From<&ServiceError> for SharedString {
    fn from(error: &ServiceError) -> Self {
        match error {
            ServiceError::Crypto(error) => crypto_use_case_error(error),
            ServiceError::Dividend(error) => dividend_use_case_error(error),
            ServiceError::Interest(error) => interest_use_case_error(error),
            ServiceError::Stock(error) => stock_use_case_error(error),
            ServiceError::Portfolio(error) => portfolio_use_case_error(error),
            ServiceError::ExchangeRefresh(error) => exchange_refresh_error(error),
            ServiceError::Import(error) => match error {
                StatementImportError::Parse(error) => import_error(error),
                StatementImportError::Repository(error) => repository_error(error),
            },
            ServiceError::RateCsv(error) => match error {
                RateCsvError::Parse(error) => csv_error(error),
                RateCsvError::Repository(error) => repository_error(error),
            },
            ServiceError::RateApi(error) => match error {
                RateApiError::Validation(error) => validation_error(error),
                RateApiError::Fetch(error) => api_error(error),
                RateApiError::Repository(error) => repository_error(error),
            },
            ServiceError::Tax(error) => match error {
                TaxError::Validation(error) => validation_error(error),
                TaxError::Repository(error) => repository_error(error),
                TaxError::Calculation(error) => match error {
                    CalculateTaxSummaryError::Crypto(error) => crypto_error(error),
                    CalculateTaxSummaryError::Dividend(error) => dividend_error(error),
                    CalculateTaxSummaryError::Interest(error) => interest_error(error),
                    CalculateTaxSummaryError::Stock(error) => stock_error(error),
                },
            },
            ServiceError::Year(error) => match error {
                YearError::Validation(error) => validation_error(error),
                YearError::Repository(error) => repository_error(error),
            },
            ServiceError::Repository(error) => repository_error(error),
            ServiceError::OpenDatabase(error) => match error {
                OpenDatabaseError::CreateDirectory(source) => {
                    t!("error.database_directory", source = source).into_owned()
                }
                OpenDatabaseError::Connect(source) => {
                    t!("error.database_open", source = source).into_owned()
                }
                OpenDatabaseError::Migrate(source) => {
                    t!("error.database_migrate", source = source).into_owned()
                }
            },
            ServiceError::DeleteDatabase(source) => {
                t!("error.database_delete", source = source).into_owned()
            }
            ServiceError::Background(source) => {
                t!("error.background", source = source).into_owned()
            }
            ServiceError::ReadPreferences(source) => {
                t!("error.read_preferences", source = source).into_owned()
            }
            ServiceError::ParsePreferences(source) => {
                t!("error.read_preferences", source = source).into_owned()
            }
            ServiceError::WritePreferences(source) => {
                t!("error.save_preferences", source = source).into_owned()
            }
            ServiceError::SerializePreferences(source) => {
                t!("error.save_preferences", source = source).into_owned()
            }
        }
        .into()
    }
}

fn validation_error(error: &ValidationError) -> String {
    match error {
        ValidationError::InvalidYear { year, min, max } => {
            t!("error.year_range", year = year, min = min, max = max)
        }
        ValidationError::InvalidTime { value, .. } => t!("error.invalid_time", value = value),
        ValidationError::NonexistentTime(time) => t!(
            "error.nonexistent_time",
            time = time.format(DATE_TIME_FORMAT)
        ),
        // English names the field in lower case inside the sentence.
        ValidationError::InvalidAmount { field, value, .. } => {
            let field = amount_field(*field);
            t!(
                "error.invalid_amount",
                field = field,
                field_lower = field.to_lowercase(),
                value = value
            )
        }
        ValidationError::InvalidSplit { field, value, .. } => {
            t!(
                "error.whole_number",
                field = amount_field(*field),
                value = value
            )
        }
        ValidationError::NotPositive { field, .. } => {
            t!("error.not_positive", field = amount_field(*field))
        }
        ValidationError::Negative { field, .. } => {
            t!("error.negative", field = amount_field(*field))
        }
        ValidationError::Empty(field) => {
            let field = match field {
                TextField::Ticker => t!("field.ticker"),
                TextField::Provider => t!("field.provider"),
            };
            t!("form.required", field = field)
        }
        ValidationError::MissingId => t!("form.required", field = t!("field.id")),
    }
    .into_owned()
}

fn amount_field(field: AmountField) -> Cow<'static, str> {
    match field {
        AmountField::Value => t!("field.value"),
        AmountField::Fee => t!("field.fee"),
        AmountField::TaxPaid => t!("field.tax_paid"),
        AmountField::Shares => t!("field.shares"),
        AmountField::Price => t!("field.price"),
        AmountField::Amount => t!("field.trade_amount"),
        AmountField::SplitBefore => t!("field.shares_before"),
        AmountField::SplitAfter => t!("field.shares_after"),
    }
}

fn crypto_use_case_error(error: &CryptoError) -> String {
    match error {
        CryptoError::Validation(error) => validation_error(error),
        CryptoError::Repository(error) => repository_error(error),
        CryptoError::DuplicateId(id) => t!("error.crypto_exists", id = id).into_owned(),
        CryptoError::NotFound(id) => t!("error.crypto_not_found", id = id).into_owned(),
        CryptoError::Calculation(error) => crypto_error(error),
        CryptoError::NothingToPreview => t!("error.nothing_to_preview").into_owned(),
    }
}

fn dividend_use_case_error(error: &DividendError) -> String {
    match error {
        DividendError::Validation(error) => validation_error(error),
        DividendError::Repository(error) => repository_error(error),
        DividendError::DuplicateId(id) => t!("error.dividend_exists", id = id).into_owned(),
        DividendError::NotFound(id) => t!("error.dividend_not_found", id = id).into_owned(),
        DividendError::Calculation(error) => dividend_error(error),
        DividendError::NothingToPreview => t!("error.nothing_to_preview").into_owned(),
    }
}

fn interest_use_case_error(error: &InterestError) -> String {
    match error {
        InterestError::Validation(error) => validation_error(error),
        InterestError::Repository(error) => repository_error(error),
        InterestError::DuplicateId(id) => t!("error.interest_exists", id = id).into_owned(),
        InterestError::NotFound(id) => t!("error.interest_not_found", id = id).into_owned(),
        InterestError::Calculation(error) => interest_error(error),
        InterestError::NothingToPreview => t!("error.nothing_to_preview").into_owned(),
    }
}

fn stock_use_case_error(error: &StockError) -> String {
    match error {
        StockError::Validation(error) => validation_error(error),
        StockError::Repository(error) => repository_error(error),
        StockError::DuplicateId(id) => t!("error.stock_exists", id = id).into_owned(),
        StockError::NotFound(id) => t!("error.stock_not_found", id = id).into_owned(),
        StockError::Calculation(error) => stock_error(error),
        StockError::NothingToPreview => t!("error.nothing_to_preview").into_owned(),
    }
}

fn portfolio_use_case_error(error: &PortfolioError) -> String {
    match error {
        PortfolioError::Repository(error) => repository_error(error),
        PortfolioError::Dividend(error) => dividend_error(error),
        PortfolioError::Interest(error) => interest_error(error),
        PortfolioError::Crypto(error) => crypto_error(error),
        PortfolioError::Stock(error) => stock_error(error),
    }
}

fn exchange_refresh_error(error: &ExchangeRefreshError) -> String {
    match error {
        ExchangeRefreshError::Download(error) => match error {
            DownloadError::Request { source } => {
                t!("error.exchanges_download", source = source).into_owned()
            }
            DownloadError::HttpStatus { status } => {
                t!("error.exchanges_status", status = status.as_u16()).into_owned()
            }
            DownloadError::Response { source } => {
                t!("error.exchanges_read", source = source).into_owned()
            }
            DownloadError::Parse(error) => match error {
                MicParseError::Csv(source) => {
                    t!("error.exchanges_parse", source = source).into_owned()
                }
                MicParseError::InvalidMic { line, value } => match line {
                    Some(line) => t!("error.invalid_mic_line", value = value, line = line),
                    None => t!("error.invalid_mic", value = value),
                }
                .into_owned(),
                MicParseError::InvalidDate { line, value, .. } => match line {
                    Some(line) => t!("error.invalid_list_date_line", value = value, line = line),
                    None => t!("error.invalid_list_date", value = value),
                }
                .into_owned(),
                MicParseError::Empty => t!("exchange.list_empty").into_owned(),
            },
        },
        ExchangeRefreshError::Repository(error) => repository_error(error),
    }
}

fn repository_error(error: &RepositoryError) -> String {
    match error {
        RepositoryError::Database(source) => t!("error.database", source = source),
        RepositoryError::Serialization(_) => t!("error.stored_value"),
        RepositoryError::Decimal(_) => t!("error.stored_amount"),
        RepositoryError::InvalidYear(year) => t!("error.year_out_of_range", year = year),
        RepositoryError::InvalidStockRecord(reason) => {
            t!("error.invalid_stock_record", reason = reason)
        }
        RepositoryError::InvalidExchange(reason) => t!("error.invalid_exchange", reason = reason),
    }
    .into_owned()
}

fn crypto_error(error: &CalculateSellBuyValuesError) -> String {
    match error {
        CalculateSellBuyValuesError::ValueConversion(source) => {
            t!("error.convert_crypto_value", error = rate_error(source))
        }
        CalculateSellBuyValuesError::FeeConversion(source) => {
            t!("error.convert_crypto_fee", error = rate_error(source))
        }
    }
    .into_owned()
}

fn dividend_error(error: &CalculateDividendTaxError) -> String {
    match error {
        CalculateDividendTaxError::DividendConversion(source) => {
            t!("error.convert_dividend_value", error = rate_error(source))
        }
        CalculateDividendTaxError::PaidTaxConversion(source) => {
            t!("error.convert_dividend_tax", error = rate_error(source))
        }
    }
    .into_owned()
}

fn interest_error(error: &CalculateInterestTaxError) -> String {
    match error {
        CalculateInterestTaxError::InterestConversion(source) => {
            t!("error.convert_interest_value", error = rate_error(source))
        }
    }
    .into_owned()
}

fn stock_error(error: &CalculateStocksError) -> String {
    match error {
        CalculateStocksError::PriceConversion(source) => {
            t!("error.convert_stock_value", error = rate_error(source))
        }
        CalculateStocksError::FeeConversion(source) => {
            t!("error.convert_stock_fee", error = rate_error(source))
        }
        CalculateStocksError::TaxPaidConversion(source) => {
            t!("error.convert_stock_tax", error = rate_error(source))
        }
    }
    .into_owned()
}

fn rate_error(error: &RateConverterError) -> String {
    match error {
        RateConverterError::NoRatesAvailable => t!("error.no_rates"),
        RateConverterError::StepLimitReached {
            steps,
            currency,
            date: at,
        } => t!(
            "error.rate_not_found",
            currency = currency,
            steps = steps,
            date = date(*at).main
        ),
    }
    .into_owned()
}

fn api_error(error: &ApiImportError) -> String {
    match error {
        ApiImportError::YearTooEarly { min_year } => t!("error.nbp_too_early", year = min_year),
        ApiImportError::FutureYear => t!("error.nbp_future"),
        ApiImportError::InvalidDate => t!("error.invalid_date"),
        ApiImportError::InvalidDateRange => t!("error.nbp_date_range"),
        ApiImportError::UnsupportedCurrency { currency } => {
            t!("error.nbp_currency", currency = currency)
        }
        ApiImportError::Request { source } => t!("error.nbp_request", source = source),
        ApiImportError::HttpStatus {
            status,
            code,
            start_date,
            end_date,
        } => t!(
            "error.nbp_status",
            status = status.as_u16(),
            code = code,
            start = date(*start_date).main,
            end = date(*end_date).main
        ),
        ApiImportError::Response { source } => t!("error.nbp_response", source = source),
    }
    .into_owned()
}

fn csv_error(error: &CsvImportError) -> String {
    match error {
        CsvImportError::InvalidExtension => t!("file.wrong_extension", extension = "csv"),
        CsvImportError::Empty => t!("error.csv_empty"),
        CsvImportError::InvalidFormat => t!("error.csv_format"),
        CsvImportError::Read(source) => t!("error.csv_read", source = source),
        CsvImportError::InvalidDate { line, .. } => t!("error.csv_date", line = line),
        CsvImportError::InvalidUnit { header, .. } => t!("error.csv_unit", header = header),
        CsvImportError::InvalidRate { line, column, .. } => {
            t!("error.csv_rate", column = column, line = line)
        }
        CsvImportError::DuplicateRate {
            currency,
            date: at,
            line,
        } => t!(
            "error.csv_duplicate",
            currency = currency,
            date = date(*at).main,
            line = line
        ),
    }
    .into_owned()
}

fn import_error(error: &ImportError) -> String {
    match error {
        ImportError::Read(source) => t!("error.import_read", source = source),
        ImportError::PdfExtract(source) => t!("error.import_pdf", source = source),
        ImportError::UnexpectedFormat { expected } => match expected {
            ImporterKind::Revolut => t!("error.import_not_revolut"),
            ImporterKind::T212 | ImporterKind::Coinbase => {
                t!("error.import_format", provider = importer_name(*expected))
            }
        },
        ImportError::MissingHeader => t!("error.import_no_header"),
        ImportError::UnexpectedHeader(header) => t!("error.import_header", header = header),
        ImportError::MissingColumn(column) => t!("error.import_column", column = column),
        ImportError::MissingSection(context) => {
            t!("error.import_section", context = import_context(context))
        }
        ImportError::MissingField { field, context } => t!(
            "error.import_missing_field",
            field = import_field(*field),
            context = import_context(context)
        ),
        ImportError::InvalidField {
            field,
            value,
            context,
        } => t!(
            "error.import_invalid_field",
            field = import_field(*field),
            context = import_context(context),
            value = value
        ),
        ImportError::CurrencyMismatch {
            field,
            context,
            expected,
            actual,
        } => t!(
            "error.import_currency",
            field = import_field(*field),
            context = import_context(context),
            expected = expected,
            actual = actual
        ),
        ImportError::CountryMismatch {
            context,
            isin,
            expected,
            actual,
        } => t!(
            "error.import_country",
            context = import_context(context),
            isin = isin,
            expected = expected,
            actual = actual
        ),
        ImportError::AmountMismatch { context, amounts } => t!(
            "error.import_amounts",
            context = import_context(context),
            gross = amounts.gross,
            tax = amounts.tax,
            net = amounts.net
        ),
        ImportError::TotalsMismatch {
            currency,
            expected,
            actual,
        } => t!(
            "error.import_totals",
            currency = currency,
            rows = import_amounts(expected),
            total = import_amounts(actual)
        ),
        ImportError::MalformedRow {
            expected,
            actual,
            row,
        } => t!(
            "error.import_row",
            expected = expected,
            actual = actual,
            row = row
        ),
        ImportError::InvalidTimestamp { value, .. } => t!("error.import_timestamp", value = value),
        ImportError::InvalidDecimal { value, .. } => t!("error.import_decimal", value = value),
        ImportError::InvalidCurrency { value, .. } => {
            t!("error.import_currency_code", value = value)
        }
        ImportError::InvalidIsin { isin, .. } => t!("error.import_isin", isin = isin),
    }
    .into_owned()
}

fn import_field(field: ImportField) -> Cow<'static, str> {
    match field {
        ImportField::Line => t!("import_field.line"),
        ImportField::TotalRow => t!("import_field.total_row"),
        ImportField::Ticker => t!("import_field.ticker"),
        ImportField::Isin => "ISIN".into(),
        ImportField::TrailingTokens => t!("import_field.trailing_tokens"),
        ImportField::GrossAmount => t!("import_field.gross_amount"),
        ImportField::WithholdingTax => t!("import_field.withholding_tax"),
        ImportField::NetAmount => t!("import_field.net_amount"),
        ImportField::LocalCurrencyRate => t!("import_field.local_currency_rate"),
        ImportField::SourceCurrency => t!("import_field.source_currency"),
        ImportField::TotalCurrency => t!("import_field.total_currency"),
    }
}

fn import_context(context: &ImportContext) -> String {
    match context {
        ImportContext::RevolutOtherIncome => t!("import_context.revolut_other_income"),
        ImportContext::RevolutRow { ticker, date: at } => match ticker {
            Some(ticker) => t!(
                "import_context.revolut_row",
                ticker = ticker,
                date = date(*at).main
            ),
            None => t!("import_context.revolut_dividend", date = date(*at).main),
        },
        ImportContext::RevolutTotal => t!("import_context.revolut_total"),
        ImportContext::T212Dividend { ticker, time } => t!(
            "import_context.t212_dividend",
            ticker = ticker,
            time = time.format(DATE_TIME_FORMAT)
        ),
    }
    .into_owned()
}

fn import_amounts(amounts: &ImportAmounts) -> String {
    t!(
        "error.import_amounts_detail",
        gross = amounts.gross,
        tax = amounts.tax,
        net = amounts.net
    )
    .into_owned()
}

pub fn country_error(error: CountryParseError) -> String {
    match error {
        CountryParseError::InvalidIsin { value } => t!("error.invalid_isin", value = value),
        CountryParseError::InvalidCode { value } => t!("error.invalid_country", value = value),
    }
    .into_owned()
}
