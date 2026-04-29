use std::collections::BTreeMap;

use anyhow::{Result as AnyhowResult, anyhow, bail, ensure};
use chrono::{Datelike, Duration, Local, NaiveDate};
use pitpls_core::{
    common::Currency,
    rate::{NbpRateProvider, RateExport},
};
use reqwest::StatusCode;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::state::AppState;

const NBP_API_BASE_URL: &str = "https://api.nbp.pl/api";
const NBP_MIN_YEAR: i32 = 2002;
const NBP_MAX_RANGE_DAYS: i64 = 93;
const NBP_CURRENCIES: [Currency; 2] = [Currency::USD, Currency::EUR];

#[derive(Serialize, Type)]
pub struct RatesViewModel {
    rows: Vec<RateDay>,
}

#[derive(Serialize, Type)]
pub struct RateDay {
    date: String,
    usd: Option<String>,
    eur: Option<String>,
}

#[derive(Default)]
struct RateValues {
    usd: Option<String>,
    eur: Option<String>,
}

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

#[tauri::command]
#[specta::specta]
pub async fn upload_rates(state: State<'_, AppState>, file: String) -> Result<u64, String> {
    let rate_provider = NbpRateProvider::load(&file)
        .await
        .map_err(|e| e.to_string())?;

    state
        .rate_repo()
        .upload(rate_provider.export())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn import_npb(state: State<'_, AppState>, year: i32) -> Result<u64, String> {
    let rates = load_nbp_rates(year).await.map_err(|e| e.to_string())?;

    state
        .rate_repo()
        .upload(rates.into_iter())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn reset_rates(state: State<'_, AppState>) -> Result<u64, String> {
    state.rate_repo().reset().await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn list_rates(state: State<'_, AppState>) -> Result<RatesViewModel, String> {
    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;

    let mut rates_by_day = BTreeMap::<_, RateValues>::new();
    for rate in rates {
        let day = rates_by_day.entry(rate.date).or_default();
        match rate.currency {
            Currency::USD => day.usd = Some(rate.rate.to_string()),
            Currency::EUR => day.eur = Some(rate.rate.to_string()),
            Currency::PLN => {}
        }
    }

    let rows = rates_by_day
        .into_iter()
        .map(|(date, rates)| RateDay {
            date: date.to_string(),
            usd: rates.usd,
            eur: rates.eur,
        })
        .collect();

    Ok(RatesViewModel { rows })
}

async fn load_nbp_rates(year: i32) -> AnyhowResult<Vec<RateExport>> {
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

fn nbp_year_ranges(year: i32) -> AnyhowResult<Vec<(NaiveDate, NaiveDate)>> {
    let today = Local::now().date_naive();

    ensure!(year >= NBP_MIN_YEAR, "NBP rates are available from 2002");
    ensure!(
        year <= today.year(),
        "Cannot import NBP rates for a future year"
    );

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
                .ok_or_else(|| anyhow!("Invalid NBP date range"))?,
            end_date,
        );
        ranges.push((cursor, range_end));

        if range_end == end_date {
            break;
        }

        cursor = range_end
            .succ_opt()
            .ok_or_else(|| anyhow!("Invalid NBP date range"))?;
    }

    Ok(ranges)
}

async fn fetch_nbp_rates(
    client: &reqwest::Client,
    currency: Currency,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> AnyhowResult<Vec<RateExport>> {
    let code = nbp_currency_code(currency)?;
    let url = format!(
        "{NBP_API_BASE_URL}/exchangerates/rates/a/{code}/{start_date}/{end_date}/?format=json",
    );
    let response = client.get(url).send().await?;
    let status = response.status();

    if status == StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !status.is_success() {
        bail!("NBP API returned {status} for {code} rates from {start_date} to {end_date}");
    }

    let response: NbpRateResponse = response.json().await?;
    Ok(response
        .rates
        .into_iter()
        .map(|rate| RateExport {
            date: rate.effective_date,
            currency,
            rate: rate.mid,
        })
        .collect())
}

fn nbp_currency_code(currency: Currency) -> AnyhowResult<&'static str> {
    match currency {
        Currency::USD => Ok("usd"),
        Currency::EUR => Ok("eur"),
        Currency::PLN => bail!("PLN rates are not imported from NBP"),
    }
}

fn date(year: i32, month: u32, day: u32) -> AnyhowResult<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| anyhow!("Invalid date"))
}
