use pitpls_core::{
    common::{Amount, Currency},
    crypto::{
        Action, CalculateSellBuyValuesError, CalculatedCrypto, Crypto, CryptoTaxData,
        calculate_sell_buy_values,
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
    Calculation(#[from] CalculateSellBuyValuesError),
    NothingToPreview,
}

pub struct CryptoFields {
    pub date: String,
    pub action: Action,
    pub value: String,
    pub value_currency: Currency,
    pub fee: String,
    pub fee_currency: Currency,
    pub provider: String,
}

pub struct CreateCryptoInput {
    pub id: Option<String>,
    pub fields: CryptoFields,
}

pub struct UpdateCryptoInput {
    pub id: String,
    pub fields: CryptoFields,
}

fn build_crypto(id: String, fields: CryptoFields) -> Result<Crypto, ValidationError> {
    let date = parse_date(&fields.date)?;
    let value_dec = parse_amount(&fields.value, AmountField::Value)?;
    let fee_dec = parse_amount(&fields.fee, AmountField::Fee)?;

    Ok(Crypto {
        id,
        date,
        action: fields.action,
        value: Amount {
            value: value_dec,
            currency: fields.value_currency,
        },
        fee: Amount {
            value: fee_dec,
            currency: fields.fee_currency,
        },
        provider: fields.provider,
    })
}

pub async fn create_crypto(app: &App, input: CreateCryptoInput) -> Result<String, Error> {
    let id = input
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let crypto = build_crypto(id.clone(), input.fields)?;

    app.db
        .crypto_repo()
        .insert(&crypto)
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

pub async fn update_crypto(app: &App, input: UpdateCryptoInput) -> Result<(), Error> {
    if input.id.trim().is_empty() {
        return Err(ValidationError::MissingId.into());
    }

    let id = input.id.clone();
    let crypto = build_crypto(id.clone(), input.fields)?;

    let rows = app.db.crypto_repo().update(&crypto).await?;
    if rows == 0 {
        return Err(Error::NotFound(id));
    }
    Ok(())
}

pub async fn delete_cryptos(app: &App, ids: Vec<String>) -> Result<u64, RepositoryError> {
    app.db.crypto_repo().delete_by_ids(&ids).await
}

pub async fn load_cryptos(app: &App, year: Option<i32>) -> Result<CryptoTaxData, Error> {
    validate_optional_year(year)?;
    let mut cryptos = app.db.crypto_repo().get_by_year(year).await?;

    cryptos.sort_unstable_by_key(|a| a.date);

    let rates = app.db.rate_repo().load_all().await?;
    let rate_provider = NbpRateProvider::new(rates);

    calculate_sell_buy_values(cryptos, &rate_provider).map_err(Error::from)
}

pub async fn preview_crypto(app: &App, crypto: Crypto) -> Result<CalculatedCrypto, Error> {
    let rate_provider = rates_for(app, crypto.date).await?;

    calculate_sell_buy_values(vec![crypto], &rate_provider)?
        .calculated
        .pop()
        .ok_or(Error::NothingToPreview)
}
