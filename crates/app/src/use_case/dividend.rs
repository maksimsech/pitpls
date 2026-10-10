use pitpls_core::{
    common::{Amount, Country, Currency},
    dividend::{
        CalculateDividendTaxError, CalculatedDividend, Dividend, DividendTaxData, calculate,
    },
    rate::NbpRateProvider,
};

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
    Calculation(#[from] CalculateDividendTaxError),
    NothingToPreview,
}

pub struct DividendFields {
    pub date: String,
    pub ticker: String,
    pub value: String,
    pub value_currency: Currency,
    pub tax_paid: String,
    pub tax_paid_currency: Currency,
    pub country: Country,
    pub provider: String,
}

pub struct CreateDividendInput {
    pub id: Option<String>,
    pub fields: DividendFields,
}

pub struct UpdateDividendInput {
    pub id: String,
    pub fields: DividendFields,
}

fn build_dividend(id: String, fields: DividendFields) -> Result<Dividend, ValidationError> {
    let date = parse_date(&fields.date)?;
    let value_dec = parse_amount(&fields.value, AmountField::Value)?;
    let tax_paid_dec = parse_amount(&fields.tax_paid, AmountField::TaxPaid)?;

    Ok(Dividend {
        id,
        date,
        ticker: fields.ticker,
        value: Amount {
            value: value_dec,
            currency: fields.value_currency,
        },
        tax_paid: Amount {
            value: tax_paid_dec,
            currency: fields.tax_paid_currency,
        },
        country: fields.country,
        provider: fields.provider,
    })
}

pub async fn create_dividend(app: &App, input: CreateDividendInput) -> Result<String, Error> {
    let id = input
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let dividend = build_dividend(id.clone(), input.fields)?;

    app.db
        .dividend_repo()
        .insert(&dividend)
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

pub async fn update_dividend(app: &App, input: UpdateDividendInput) -> Result<(), Error> {
    if input.id.trim().is_empty() {
        return Err(ValidationError::MissingId.into());
    }

    let id = input.id.clone();
    let dividend = build_dividend(id.clone(), input.fields)?;

    let rows = app.db.dividend_repo().update(&dividend).await?;
    if rows == 0 {
        return Err(Error::NotFound(id));
    }
    Ok(())
}

pub async fn delete_dividends(app: &App, ids: Vec<String>) -> Result<u64, RepositoryError> {
    app.db.dividend_repo().delete_by_ids(&ids).await
}

pub async fn load_dividends(app: &App, year: Option<i32>) -> Result<DividendTaxData, Error> {
    validate_optional_year(year)?;
    let mut dividends = app.db.dividend_repo().get_by_year(year).await?;

    dividends.sort_unstable_by_key(|a| a.date);

    let rates = app.db.rate_repo().load_all().await?;
    let rate_provider = NbpRateProvider::new(rates);
    let dividend_rounding = app.db.settings_repo().load_dividend_rounding().await?;

    calculate(dividends, &rate_provider, dividend_rounding).map_err(Error::from)
}

pub async fn preview_dividend(app: &App, dividend: Dividend) -> Result<CalculatedDividend, Error> {
    let rate_provider = rates_for(app, dividend.date).await?;
    let dividend_rounding = app.db.settings_repo().load_dividend_rounding().await?;

    calculate(vec![dividend], &rate_provider, dividend_rounding)?
        .calculated
        .pop()
        .ok_or(Error::NothingToPreview)
}
