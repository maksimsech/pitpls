use pitpls_core::{
    common::{Amount, Currency},
    interest::{
        CalculateInterestTaxError, CalculatedInterest, Interest, InterestTaxData, calculate,
    },
    rate::NbpRateProvider,
};
use serde::Deserialize;

use super::rates_for;
use super::validation::{
    AmountField, Error as ValidationError, parse_amount, parse_date, validate_optional_year,
};
use crate::App;
use pitpls_db::RepositoryError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    Validation(#[from] ValidationError),
    Repository(#[from] RepositoryError),
    DuplicateId(String),
    NotFound(String),
    Calculation(#[from] CalculateInterestTaxError),
    NothingToPreview,
}

#[derive(Deserialize)]
pub struct CreateInterestInput {
    pub id: Option<String>,
    pub date: String,
    pub value: String,
    pub value_currency: Currency,
    pub provider: String,
}

#[derive(Deserialize)]
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
) -> Result<Interest, ValidationError> {
    let date = parse_date(date)?;
    let value_dec = parse_amount(value, AmountField::Value)?;

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

pub async fn create_interest(app: &App, input: CreateInterestInput) -> Result<String, Error> {
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
        .map_err(|error| {
            if error.is_unique_violation() {
                Error::DuplicateId(id.clone())
            } else {
                Error::Repository(error)
            }
        })?;

    Ok(id)
}

pub async fn update_interest(app: &App, input: UpdateInterestInput) -> Result<(), Error> {
    if input.id.trim().is_empty() {
        return Err(ValidationError::MissingId.into());
    }

    let id = input.id.clone();
    let interest = build_interest(
        id.clone(),
        &input.date,
        &input.value,
        input.value_currency,
        input.provider,
    )?;

    let rows = app.db.interest_repo().update(&interest).await?;
    if rows == 0 {
        return Err(Error::NotFound(id));
    }
    Ok(())
}

pub async fn delete_interests(app: &App, ids: Vec<String>) -> Result<u64, RepositoryError> {
    app.db.interest_repo().delete_by_ids(&ids).await
}

pub async fn load_interests(app: &App, year: Option<i32>) -> Result<InterestTaxData, Error> {
    validate_optional_year(year)?;
    let mut interests = app.db.interest_repo().get_by_year(year).await?;

    interests.sort_unstable_by_key(|a| a.date);

    let rates = app.db.rate_repo().load_all().await?;
    let rate_provider = NbpRateProvider::new(rates);

    calculate(interests, &rate_provider).map_err(Error::from)
}

pub async fn preview_interest(app: &App, interest: Interest) -> Result<CalculatedInterest, Error> {
    let rate_provider = rates_for(app, interest.date).await?;

    calculate(vec![interest], &rate_provider)?
        .calculated
        .pop()
        .ok_or(Error::NothingToPreview)
}
