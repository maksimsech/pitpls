use std::str::FromStr;

use chrono::NaiveDate;
use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::{Dividend, DividendTaxData, calculate},
    rate::NbpRateProvider,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Deserialize, Type)]
pub struct CreateDividendInput {
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

#[derive(Deserialize, Type)]
pub struct UpdateDividendInput {
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

fn build_dividend(
    id: String,
    date: &str,
    ticker: String,
    value: &str,
    value_currency: Currency,
    tax_paid: &str,
    tax_paid_currency: Currency,
    country: Country,
    provider: String,
) -> Result<Dividend, String> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| "Invalid date (expected YYYY-MM-DD)".to_string())?;
    let value_dec = Decimal::from_str(value).map_err(|_| format!("Invalid value: {value}"))?;
    let tax_paid_dec =
        Decimal::from_str(tax_paid).map_err(|_| format!("Invalid tax_paid: {tax_paid}"))?;

    Ok(Dividend {
        id,
        date,
        ticker,
        value: Amount {
            value: value_dec,
            currency: value_currency,
        },
        tax_paid: Amount {
            value: tax_paid_dec,
            currency: tax_paid_currency,
        },
        country,
        provider,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn create_dividend(
    state: State<'_, AppState>,
    input: CreateDividendInput,
) -> Result<String, String> {
    let id = input
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let dividend = build_dividend(
        id.clone(),
        &input.date,
        input.ticker,
        &input.value,
        input.value_currency,
        &input.tax_paid,
        input.tax_paid_currency,
        input.country,
        input.provider,
    )?;

    match state.dividend_repo().save(&[dividend]).await {
        Ok(_) => Ok(id),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE constraint failed") {
                Err(format!("Dividend with ID '{id}' already exists"))
            } else {
                Err(msg)
            }
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn update_dividend(
    state: State<'_, AppState>,
    input: UpdateDividendInput,
) -> Result<(), String> {
    if input.id.trim().is_empty() {
        return Err("ID is required".into());
    }

    let id = input.id.clone();
    let dividend = build_dividend(
        id.clone(),
        &input.date,
        input.ticker,
        &input.value,
        input.value_currency,
        &input.tax_paid,
        input.tax_paid_currency,
        input.country,
        input.provider,
    )?;

    let rows = state
        .dividend_repo()
        .update(&dividend)
        .await
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err(format!("Dividend with ID '{id}' not found"));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_dividends(state: State<'_, AppState>, ids: Vec<String>) -> Result<u64, String> {
    state
        .dividend_repo()
        .delete_by_ids(&ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn load_dividends(state: State<'_, AppState>) -> Result<DividendTaxData, String> {
    let mut dividends = state
        .dividend_repo()
        .get_all()
        .await
        .map_err(|e| e.to_string())?;

    dividends.sort_unstable_by(|a, b| a.date.cmp(&b.date));

    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;
    let rate_provider = NbpRateProvider::new(rates);

    calculate(dividends, &rate_provider).map_err(|e| e.to_string())
}
