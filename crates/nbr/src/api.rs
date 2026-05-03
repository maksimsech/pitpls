use chrono::{Datelike, Duration, Local, NaiveDate};
use pitpls_core::{common::Currency, rate::Rate};
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

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

const NBP_API_BASE_URL: &str = "https://api.nbp.pl/api";
const NBP_MIN_YEAR: i32 = 2002;
const NBP_MAX_RANGE_DAYS: i64 = 93;
const NBP_TABLE_A: &str = "a";

#[derive(Clone, Copy)]
struct NbpCurrency {
    currency: Currency,
    code: &'static str,
    unit: u32,
}

const NBP_CURRENCIES: [NbpCurrency; 33] = [
    NbpCurrency {
        currency: Currency::THB,
        code: "THB",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::USD,
        code: "USD",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::AUD,
        code: "AUD",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::HKD,
        code: "HKD",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::CAD,
        code: "CAD",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::NZD,
        code: "NZD",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::SGD,
        code: "SGD",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::EUR,
        code: "EUR",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::HUF,
        code: "HUF",
        unit: 100,
    },
    NbpCurrency {
        currency: Currency::CHF,
        code: "CHF",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::GBP,
        code: "GBP",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::UAH,
        code: "UAH",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::JPY,
        code: "JPY",
        unit: 100,
    },
    NbpCurrency {
        currency: Currency::CZK,
        code: "CZK",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::DKK,
        code: "DKK",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::ISK,
        code: "ISK",
        unit: 100,
    },
    NbpCurrency {
        currency: Currency::NOK,
        code: "NOK",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::SEK,
        code: "SEK",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::RON,
        code: "RON",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::BGN,
        code: "BGN",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::TRY,
        code: "TRY",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::ILS,
        code: "ILS",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::CLP,
        code: "CLP",
        unit: 100,
    },
    NbpCurrency {
        currency: Currency::PHP,
        code: "PHP",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::MXN,
        code: "MXN",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::ZAR,
        code: "ZAR",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::BRL,
        code: "BRL",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::MYR,
        code: "MYR",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::IDR,
        code: "IDR",
        unit: 10000,
    },
    NbpCurrency {
        currency: Currency::INR,
        code: "INR",
        unit: 100,
    },
    NbpCurrency {
        currency: Currency::KRW,
        code: "KRW",
        unit: 100,
    },
    NbpCurrency {
        currency: Currency::CNY,
        code: "CNY",
        unit: 1,
    },
    NbpCurrency {
        currency: Currency::XDR,
        code: "XDR",
        unit: 1,
    },
];

#[derive(Deserialize)]
struct NbpTableResponse {
    #[serde(rename = "effectiveDate")]
    effective_date: NaiveDate,
    rates: Vec<NbpTableRate>,
}

#[derive(Deserialize)]
struct NbpTableRate {
    code: String,
    mid: Decimal,
}

pub async fn load_api_rates(client: &Client, year: i32) -> Result<Vec<Rate>, ApiImportError> {
    let ranges = nbp_year_ranges(year)?;
    let mut rates = Vec::new();

    for (start_date, end_date) in &ranges {
        rates.extend(fetch_nbp_rates(client, *start_date, *end_date).await?);
    }

    Ok(rates)
}

fn nbp_year_ranges(year: i32) -> Result<Vec<(NaiveDate, NaiveDate)>, ApiImportError> {
    let today = Local::now().date_naive();

    if year < NBP_MIN_YEAR {
        return Err(ApiImportError::YearTooEarly {
            min_year: NBP_MIN_YEAR,
        });
    }

    if year > today.year() {
        return Err(ApiImportError::FutureYear);
    }

    let mut cursor = if year == NBP_MIN_YEAR {
        date(year, 1, 2)?
    } else {
        date(year, 1, 1)?
    };
    let year_end = date(year, 12, 31)?;
    let end_date = if year == today.year() {
        std::cmp::min(today, year_end)
    } else {
        year_end
    };

    let mut ranges = Vec::new();
    while cursor <= end_date {
        let range_end = std::cmp::min(
            cursor
                .checked_add_signed(Duration::days(NBP_MAX_RANGE_DAYS - 1))
                .ok_or(ApiImportError::InvalidDateRange)?,
            end_date,
        );
        ranges.push((cursor, range_end));

        if range_end == end_date {
            break;
        }

        cursor = range_end
            .succ_opt()
            .ok_or(ApiImportError::InvalidDateRange)?;
    }

    Ok(ranges)
}

async fn fetch_nbp_rates(
    client: &Client,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<Rate>, ApiImportError> {
    let url = format!(
        "{NBP_API_BASE_URL}/exchangerates/tables/{NBP_TABLE_A}/{start_date}/{end_date}/?format=json",
    );
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| ApiImportError::Request { source })?;
    let status = response.status();

    if status == StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !status.is_success() {
        return Err(ApiImportError::HttpStatus {
            status,
            code: "table A",
            start_date,
            end_date,
        });
    }

    let response: Vec<NbpTableResponse> = response
        .json()
        .await
        .map_err(|source| ApiImportError::Response { source })?;
    Ok(response
        .into_iter()
        .flat_map(|table| {
            table.rates.into_iter().filter_map(move |rate| {
                let currency = nbp_currency(&rate.code)?;

                Some(Rate {
                    date: table.effective_date,
                    currency: currency.currency,
                    rate: rate.mid / Decimal::from(currency.unit),
                })
            })
        })
        .collect())
}

fn nbp_currency(code: &str) -> Option<&'static NbpCurrency> {
    NBP_CURRENCIES
        .iter()
        .find(|currency| currency.code.eq_ignore_ascii_case(code))
}

fn date(year: i32, month: u32, day: u32) -> Result<NaiveDate, ApiImportError> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or(ApiImportError::InvalidDate)
}
