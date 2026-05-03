use chrono::{Datelike, Duration, Local, NaiveDate};
use pitpls_core::{common::Currency, rate::Rate};
use reqwest::StatusCode;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::ApiImportError;

const NBP_API_BASE_URL: &str = "https://api.nbp.pl/api";
const NBP_MIN_YEAR: i32 = 2002;
const NBP_MAX_RANGE_DAYS: i64 = 93;
const NBP_CURRENCIES: [Currency; 2] = [Currency::USD, Currency::EUR];

#[derive(Deserialize)]
struct NbpRateResponse {
    rates: Vec<NbpRate>,
}

#[derive(Deserialize)]
struct NbpRate {
    #[serde(rename = "effectiveDate")]
    effective_date: NaiveDate,
    mid: Decimal,
}

pub async fn load_api_rates(year: i32) -> Result<Vec<Rate>, ApiImportError> {
    let ranges = nbp_year_ranges(year)?;
    let client = reqwest::Client::new();
    let mut rates = Vec::new();

    for currency in NBP_CURRENCIES {
        for (start_date, end_date) in &ranges {
            rates.extend(fetch_nbp_rates(&client, currency, *start_date, *end_date).await?);
        }
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
    client: &reqwest::Client,
    currency: Currency,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<Rate>, ApiImportError> {
    let code = nbp_currency_code(currency)?;
    let url = format!(
        "{NBP_API_BASE_URL}/exchangerates/rates/a/{code}/{start_date}/{end_date}/?format=json",
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
            code,
            start_date,
            end_date,
        });
    }

    let response: NbpRateResponse = response
        .json()
        .await
        .map_err(|source| ApiImportError::Response { source })?;
    Ok(response
        .rates
        .into_iter()
        .map(|rate| Rate {
            date: rate.effective_date,
            currency,
            rate: rate.mid,
        })
        .collect())
}

fn nbp_currency_code(currency: Currency) -> Result<&'static str, ApiImportError> {
    match currency {
        Currency::USD => Ok("usd"),
        Currency::EUR => Ok("eur"),
        Currency::PLN => Err(ApiImportError::UnsupportedCurrency {
            currency: currency.to_string(),
        }),
    }
}

fn date(year: i32, month: u32, day: u32) -> Result<NaiveDate, ApiImportError> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or(ApiImportError::InvalidDate)
}
