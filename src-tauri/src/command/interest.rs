use std::str::FromStr;

use chrono::NaiveDate;
use pitpls_core::{
    common::{Amount, Currency},
    interest::{Interest, InterestTaxData, calculate},
    rate::NbpRateProvider,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Deserialize, Type)]
pub struct CreateInterestInput {
    pub id: Option<String>,
    pub date: String,
    pub value: String,
    pub value_currency: Currency,
    pub provider: String,
}

#[derive(Deserialize, Type)]
pub struct UpdateInterestInput {
    pub id: String,
    pub date: String,
    pub value: String,
    pub value_currency: Currency,
    pub provider: String,
}

fn build_interest(
    id: String,
    date: &str,
    value: &str,
    value_currency: Currency,
    provider: String,
) -> Result<Interest, String> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| "Invalid date (expected YYYY-MM-DD)".to_string())?;
    let value_dec = Decimal::from_str(value).map_err(|_| format!("Invalid value: {value}"))?;

    Ok(Interest {
        id,
        date,
        value: Amount {
            value: value_dec,
            currency: value_currency,
        },
        provider,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn create_interest(
    state: State<'_, AppState>,
    input: CreateInterestInput,
) -> Result<String, String> {
    let id = input
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let interest = build_interest(
        id.clone(),
        &input.date,
        &input.value,
        input.value_currency,
        input.provider,
    )?;

    match state.interest_repo().save(&[interest]).await {
        Ok(_) => Ok(id),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE constraint failed") {
                Err(format!("Interest with ID '{id}' already exists"))
            } else {
                Err(msg)
            }
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn update_interest(
    state: State<'_, AppState>,
    input: UpdateInterestInput,
) -> Result<(), String> {
    if input.id.trim().is_empty() {
        return Err("ID is required".into());
    }

    let id = input.id.clone();
    let interest = build_interest(
        id.clone(),
        &input.date,
        &input.value,
        input.value_currency,
        input.provider,
    )?;

    let rows = state
        .interest_repo()
        .update(&interest)
        .await
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err(format!("Interest with ID '{id}' not found"));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_interests(state: State<'_, AppState>, ids: Vec<String>) -> Result<u64, String> {
    state
        .interest_repo()
        .delete_by_ids(&ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn load_interests(
    state: State<'_, AppState>,
    year: Option<i32>,
) -> Result<InterestTaxData, String> {
    let mut interests = state
        .interest_repo()
        .get_by_year(year)
        .await
        .map_err(|e| e.to_string())?;

    interests.sort_unstable_by(|a, b| a.date.cmp(&b.date));

    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;
    let rate_provider = NbpRateProvider::new(rates);

    calculate(interests, &rate_provider).map_err(|e| e.to_string())
}
