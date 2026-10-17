use chrono::{Datelike, Duration, Local, NaiveDate};
use pitpls_core::{common::Currency, rate::Rate};
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum ApiImportError {
    YearTooEarly {
        min_year: i32,
    },
    FutureYear,
    InvalidDate,
    InvalidDateRange,
    UnsupportedCurrency {
        currency: Currency,
    },
    Request {
        #[source]
        source: reqwest::Error,
    },
    HttpStatus {
        status: StatusCode,
        code: &'static str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
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
}

const NBP_CURRENCIES: [NbpCurrency; 33] = [
    NbpCurrency {
        currency: Currency::THB,
        code: "THB",
    },
    NbpCurrency {
        currency: Currency::USD,
        code: "USD",
    },
    NbpCurrency {
        currency: Currency::AUD,
        code: "AUD",
    },
    NbpCurrency {
        currency: Currency::HKD,
        code: "HKD",
    },
    NbpCurrency {
        currency: Currency::CAD,
        code: "CAD",
    },
    NbpCurrency {
        currency: Currency::NZD,
        code: "NZD",
    },
    NbpCurrency {
        currency: Currency::SGD,
        code: "SGD",
    },
    NbpCurrency {
        currency: Currency::EUR,
        code: "EUR",
    },
    NbpCurrency {
        currency: Currency::HUF,
        code: "HUF",
    },
    NbpCurrency {
        currency: Currency::CHF,
        code: "CHF",
    },
    NbpCurrency {
        currency: Currency::GBP,
        code: "GBP",
    },
    NbpCurrency {
        currency: Currency::UAH,
        code: "UAH",
    },
    NbpCurrency {
        currency: Currency::JPY,
        code: "JPY",
    },
    NbpCurrency {
        currency: Currency::CZK,
        code: "CZK",
    },
    NbpCurrency {
        currency: Currency::DKK,
        code: "DKK",
    },
    NbpCurrency {
        currency: Currency::ISK,
        code: "ISK",
    },
    NbpCurrency {
        currency: Currency::NOK,
        code: "NOK",
    },
    NbpCurrency {
        currency: Currency::SEK,
        code: "SEK",
    },
    NbpCurrency {
        currency: Currency::RON,
        code: "RON",
    },
    NbpCurrency {
        currency: Currency::BGN,
        code: "BGN",
    },
    NbpCurrency {
        currency: Currency::TRY,
        code: "TRY",
    },
    NbpCurrency {
        currency: Currency::ILS,
        code: "ILS",
    },
    NbpCurrency {
        currency: Currency::CLP,
        code: "CLP",
    },
    NbpCurrency {
        currency: Currency::PHP,
        code: "PHP",
    },
    NbpCurrency {
        currency: Currency::MXN,
        code: "MXN",
    },
    NbpCurrency {
        currency: Currency::ZAR,
        code: "ZAR",
    },
    NbpCurrency {
        currency: Currency::BRL,
        code: "BRL",
    },
    NbpCurrency {
        currency: Currency::MYR,
        code: "MYR",
    },
    NbpCurrency {
        currency: Currency::IDR,
        code: "IDR",
    },
    NbpCurrency {
        currency: Currency::INR,
        code: "INR",
    },
    NbpCurrency {
        currency: Currency::KRW,
        code: "KRW",
    },
    NbpCurrency {
        currency: Currency::CNY,
        code: "CNY",
    },
    NbpCurrency {
        currency: Currency::XDR,
        code: "XDR",
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
    let ranges = nbp_year_ranges(year, Local::now().date_naive())?;
    let mut rates = Vec::new();

    for (start_date, end_date) in &ranges {
        rates.extend(fetch_nbp_rates(client, *start_date, *end_date).await?);
    }

    Ok(rates)
}

fn nbp_year_ranges(
    year: i32,
    today: NaiveDate,
) -> Result<Vec<(NaiveDate, NaiveDate)>, ApiImportError> {
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

    let tables: Vec<NbpTableResponse> = response
        .json()
        .await
        .map_err(|source| ApiImportError::Response { source })?;
    Ok(table_rates(tables))
}

fn table_rates(tables: Vec<NbpTableResponse>) -> Vec<Rate> {
    tables
        .into_iter()
        .flat_map(|table| {
            table.rates.into_iter().filter_map(move |rate| {
                let currency = nbp_currency(&rate.code)?;

                Some(Rate {
                    date: table.effective_date,
                    currency: currency.currency,
                    rate: rate.mid,
                })
            })
        })
        .collect()
}

fn nbp_currency(code: &str) -> Option<&'static NbpCurrency> {
    NBP_CURRENCIES
        .iter()
        .find(|currency| currency.code.eq_ignore_ascii_case(code))
}

fn date(year: i32, month: u32, day: u32) -> Result<NaiveDate, ApiImportError> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or(ApiImportError::InvalidDate)
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn mid_is_already_per_unit() {
        let tables: Vec<NbpTableResponse> = serde_json::from_str(
            r#"[{
                "table": "A",
                "no": "042/A/NBP/2025",
                "effectiveDate": "2025-03-03",
                "rates": [
                    {"currency": "dolar amerykański", "code": "USD", "mid": 4.0112},
                    {"currency": "jen (Japonia)", "code": "JPY", "mid": 0.026681},
                    {"currency": "rupia indonezyjska", "code": "IDR", "mid": 0.0002434},
                    {"currency": "złoto", "code": "XAU", "mid": 300.0}
                ]
            }]"#,
        )
        .unwrap();

        let rates = table_rates(tables)
            .into_iter()
            .map(|rate| (rate.date, rate.currency, rate.rate))
            .collect::<Vec<_>>();

        assert_eq!(
            rates,
            [
                (date(2025, 3, 3), Currency::USD, dec!(4.0112)),
                (date(2025, 3, 3), Currency::JPY, dec!(0.026681)),
                (date(2025, 3, 3), Currency::IDR, dec!(0.0002434)),
            ]
        );
    }

    #[test]
    fn year_splits_into_93_day_ranges() {
        let today = date(2026, 10, 17);

        assert_eq!(
            nbp_year_ranges(2024, today).unwrap(),
            [
                (date(2024, 1, 1), date(2024, 4, 2)),
                (date(2024, 4, 3), date(2024, 7, 4)),
                (date(2024, 7, 5), date(2024, 10, 5)),
                (date(2024, 10, 6), date(2024, 12, 31)),
            ]
        );
        assert_eq!(
            nbp_year_ranges(2026, today).unwrap().last(),
            Some(&(date(2026, 10, 7), today))
        );
        assert_eq!(nbp_year_ranges(2002, today).unwrap()[0].0, date(2002, 1, 2));
        assert!(matches!(
            nbp_year_ranges(2001, today),
            Err(ApiImportError::YearTooEarly { min_year: 2002 })
        ));
        assert!(matches!(
            nbp_year_ranges(2027, today),
            Err(ApiImportError::FutureYear)
        ));
    }
}
