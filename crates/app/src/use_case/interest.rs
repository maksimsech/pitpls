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

use super::{duplicate_id_error, error_message, validate_optional_year};
use crate::App;

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

pub async fn create_interest(app: &App, input: CreateInterestInput) -> Result<String, String> {
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

    app.db
        .interest_repo()
        .insert(&interest)
        .await
        .map_err(|error| duplicate_id_error(error, "Interest", &id))?;

    Ok(id)
}

pub async fn update_interest(app: &App, input: UpdateInterestInput) -> Result<(), String> {
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

    let rows = app
        .db
        .interest_repo()
        .update(&interest)
        .await
        .map_err(error_message)?;
    if rows == 0 {
        return Err(format!("Interest with ID '{id}' not found"));
    }
    Ok(())
}

pub async fn delete_interests(app: &App, ids: Vec<String>) -> Result<u64, String> {
    app.db
        .interest_repo()
        .delete_by_ids(&ids)
        .await
        .map_err(error_message)
}

pub async fn load_interests(app: &App, year: Option<i32>) -> Result<InterestTaxData, String> {
    validate_optional_year(year)?;
    let mut interests = app
        .db
        .interest_repo()
        .get_by_year(year)
        .await
        .map_err(error_message)?;

    interests.sort_unstable_by(|a, b| a.date.cmp(&b.date));

    let rates = app.db.rate_repo().load_all().await.map_err(error_message)?;
    let rate_provider = NbpRateProvider::new(rates);

    calculate(interests, &rate_provider).map_err(error_message)
}
