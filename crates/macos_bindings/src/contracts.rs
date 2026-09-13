//! UniFFI descriptions of the existing Rust contracts. No domain logic lives here.
use chrono::NaiveDate;
use pitpls_app::use_case::{
    crypto::*, dividend::*, import::ImportResult, interest::*, rate::*, warnings::Warnings,
};
use pitpls_core::{
    common::{Amount, Country, Currency},
    crypto::*,
    dividend::*,
    interest::*,
    settings::*,
    summary::*,
};
use pitpls_importers::model::ImporterKind;
use std::str::FromStr;
type DecimalAmount = rust_decimal::Decimal;

uniffi::custom_type!(DecimalAmount, String, {
    remote,
    lower: |value| value.to_string(),
    try_lift: |value| DecimalAmount::from_str(&value).map_err(|error| crate::NativeError::from(error.to_string()).into()),
});
uniffi::custom_type!(NaiveDate, String, {
    remote,
    lower: |value| value.to_string(),
    try_lift: |value| NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|error| crate::NativeError::from(error.to_string()).into()),
});
uniffi::custom_type!(Country, String, {
    remote,
    lower: |value| value.to_string(),
    try_lift: |value| Country::from_str(&value).map_err(|error| crate::NativeError::from(error).into()),
});

#[uniffi::remote(Record)]
struct Amount {
    pub value: DecimalAmount,
    pub currency: Currency,
}

#[uniffi::remote(Enum)]
enum Action {
    FiatBuy,
    FiatSell,
}

#[uniffi::remote(Record)]
struct CalculatedCrypto {
    pub id: String,
    pub value: Amount,
    pub calculated_value: DecimalAmount,
    pub fee: Amount,
    pub calculated_fee: DecimalAmount,
    pub action: Action,
    pub date: NaiveDate,
    pub nbp_date: NaiveDate,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct CryptoTaxData {
    pub income: DecimalAmount,
    pub costs: DecimalAmount,
    pub calculated: Vec<CalculatedCrypto>,
}

#[uniffi::remote(Record)]
struct CalculatedDividend {
    pub id: String,
    pub date: NaiveDate,
    pub nbp_date: NaiveDate,
    pub ticker: String,
    pub value: Amount,
    pub calculated_value: DecimalAmount,
    pub calculated_to_pay: DecimalAmount,
    pub tax_paid: Amount,
    pub calculated_tax_paid: DecimalAmount,
    pub max_tax_paid: DecimalAmount,
    pub used_tax_paid: DecimalAmount,
    pub country: Country,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct DividendTaxData {
    pub to_pay: DecimalAmount,
    pub paid: DecimalAmount,
    pub income: DecimalAmount,
    pub calculated: Vec<CalculatedDividend>,
}

#[uniffi::remote(Record)]
struct CalculatedInterest {
    pub id: String,
    pub date: NaiveDate,
    pub nbp_date: NaiveDate,
    pub value: Amount,
    pub calculated_value: DecimalAmount,
    pub to_pay: DecimalAmount,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct InterestTaxData {
    pub to_pay: DecimalAmount,
    pub income: DecimalAmount,
    pub calculated: Vec<CalculatedInterest>,
}

#[uniffi::remote(Enum)]
enum DividendRounding {
    SumToGroszy,
    SumToPayToZlote,
    SumBothToZlote,
    AllToZlote,
}

#[uniffi::remote(Record)]
struct Settings {
    pub dividend_rounding: DividendRounding,
}

#[uniffi::remote(Record)]
struct CryptoTaxSummary {
    pub income: DecimalAmount,
    pub costs: DecimalAmount,
}

#[uniffi::remote(Record)]
struct ForeignTaxSummary {
    pub income: DecimalAmount,
    pub tax_to_pay: DecimalAmount,
    pub tax_paid: DecimalAmount,
}

#[uniffi::remote(Record)]
struct TaxSummary {
    pub crypto: CryptoTaxSummary,
    pub foreign: ForeignTaxSummary,
}

#[uniffi::remote(Record)]
struct CreateCryptoInput {
    pub id: Option<String>,
    pub date: String,
    pub action: Action,
    pub value: String,
    pub value_currency: Currency,
    pub fee: String,
    pub fee_currency: Currency,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct UpdateCryptoInput {
    pub id: String,
    pub date: String,
    pub action: Action,
    pub value: String,
    pub value_currency: Currency,
    pub fee: String,
    pub fee_currency: Currency,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct CreateDividendInput {
    pub id: Option<String>,
    pub date: String,
    pub ticker: String,
    pub value: String,
    pub value_currency: Currency,
    pub tax_paid: String,
    pub tax_paid_currency: Currency,
    pub country: Country,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct UpdateDividendInput {
    pub id: String,
    pub date: String,
    pub ticker: String,
    pub value: String,
    pub value_currency: Currency,
    pub tax_paid: String,
    pub tax_paid_currency: Currency,
    pub country: Country,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct CreateInterestInput {
    pub id: Option<String>,
    pub date: String,
    pub value: String,
    pub value_currency: Currency,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct UpdateInterestInput {
    pub id: String,
    pub date: String,
    pub value: String,
    pub value_currency: Currency,
    pub provider: String,
}

#[uniffi::remote(Record)]
struct RatesViewModel {
    pub currencies: Vec<Currency>,
    pub rows: Vec<RateDay>,
}

#[uniffi::remote(Record)]
struct RateDay {
    pub date: String,
    pub rates: Vec<RateValue>,
}

#[uniffi::remote(Record)]
struct RateValue {
    pub currency: Currency,
    pub rate: String,
}

#[uniffi::remote(Record)]
struct ImportResult {
    pub dividends: u64,
    pub cryptos: u64,
    pub interests: u64,
}

#[uniffi::remote(Record)]
struct Warnings {
    pub rates_empty: bool,
    pub has_records_in_year: bool,
}

#[uniffi::remote(Enum)]
enum Currency {
    THB,
    USD,
    AUD,
    HKD,
    CAD,
    NZD,
    SGD,
    EUR,
    HUF,
    CHF,
    GBP,
    UAH,
    JPY,
    CZK,
    DKK,
    ISK,
    NOK,
    SEK,
    RON,
    BGN,
    TRY,
    ILS,
    CLP,
    PHP,
    MXN,
    ZAR,
    BRL,
    MYR,
    IDR,
    INR,
    KRW,
    CNY,
    XDR,
    PLN,
}

#[uniffi::remote(Enum)]
enum ImporterKind {
    T212,
    Revolut,
    Coinbase,
}
