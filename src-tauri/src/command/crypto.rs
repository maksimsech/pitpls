use std::str::FromStr;

use chrono::NaiveDate;
use pitpls_core::{
    common::{Amount, Currency},
    crypto::{Action, Crypto, CryptoTaxData, calculate_sell_buy_values},
    rate::NbpRateProvider,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Deserialize, Type)]
pub struct CreateCryptoInput {
    pub id: Option<String>,
    pub date: String,
    pub action: Action,
    pub value: String,
    pub value_currency: Currency,
    pub fee: String,
    pub fee_currency: Currency,
    pub provider: String,
}

#[derive(Deserialize, Type)]
pub struct UpdateCryptoInput {
    pub id: String,
    pub date: String,
    pub action: Action,
    pub value: String,
    pub value_currency: Currency,
    pub fee: String,
    pub fee_currency: Currency,
    pub provider: String,
}

fn build_crypto(
    id: String,
    date: &str,
    action: Action,
    value: &str,
    value_currency: Currency,
    fee: &str,
    fee_currency: Currency,
    provider: String,
) -> Result<Crypto, String> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| "Invalid date (expected YYYY-MM-DD)".to_string())?;
    let value_dec = Decimal::from_str(value).map_err(|_| format!("Invalid value: {value}"))?;
    let fee_dec = Decimal::from_str(fee).map_err(|_| format!("Invalid fee: {fee}"))?;

    Ok(Crypto {
        id,
        date,
        action,
        value: Amount {
            value: value_dec,
            currency: value_currency,
        },
        fee: Amount {
            value: fee_dec,
            currency: fee_currency,
        },
        provider,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn create_crypto(
    state: State<'_, AppState>,
    input: CreateCryptoInput,
) -> Result<String, String> {
    let id = input
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let crypto = build_crypto(
        id.clone(),
        &input.date,
        input.action,
        &input.value,
        input.value_currency,
        &input.fee,
        input.fee_currency,
        input.provider,
    )?;

    match state.crypto_repo().save(&[crypto]).await {
        Ok(_) => Ok(id),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("UNIQUE constraint failed") {
                Err(format!("Crypto with ID '{id}' already exists"))
            } else {
                Err(msg)
            }
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn update_crypto(
    state: State<'_, AppState>,
    input: UpdateCryptoInput,
) -> Result<(), String> {
    if input.id.trim().is_empty() {
        return Err("ID is required".into());
    }

    let id = input.id.clone();
    let crypto = build_crypto(
        id.clone(),
        &input.date,
        input.action,
        &input.value,
        input.value_currency,
        &input.fee,
        input.fee_currency,
        input.provider,
    )?;

    let rows = state
        .crypto_repo()
        .update(&crypto)
        .await
        .map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err(format!("Crypto with ID '{id}' not found"));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_cryptos(state: State<'_, AppState>, ids: Vec<String>) -> Result<u64, String> {
    state
        .crypto_repo()
        .delete_by_ids(&ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn load_cryptos(
    state: State<'_, AppState>,
    year: Option<i32>,
) -> Result<CryptoTaxData, String> {
    let mut cryptos = state
        .crypto_repo()
        .get_by_year(year)
        .await
        .map_err(|e| e.to_string())?;

    cryptos.sort_unstable_by(|a, b| a.date.cmp(&b.date));

    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;
    let rate_provider = NbpRateProvider::new(rates);

    calculate_sell_buy_values(cryptos, &rate_provider).map_err(|e| e.to_string())
}
