//! Thin, typed native adapter over the same use cases as Tauri.
use pitpls_app::use_case::{
    crypto::{self, CreateCryptoInput, UpdateCryptoInput},
    dividend::{self, CreateDividendInput, UpdateDividendInput},
    import::{self, ImportResult},
    interest::{self, CreateInterestInput, UpdateInterestInput},
    rate::{self, RatesViewModel},
    settings, tax,
    warnings::{self, Warnings},
    year as year_use_case,
};
use pitpls_core::{
    crypto::CryptoTaxData, dividend::DividendTaxData, interest::InterestTaxData,
    settings::Settings, summary::TaxSummary,
};
use pitpls_importers::model::ImporterKind;

use pitpls_app::App;
use pitpls_db::Database;
use std::sync::Arc;
mod contracts;
uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum NativeError {
    #[error("{message}")]
    Failed { message: String },
}
impl From<String> for NativeError {
    fn from(message: String) -> Self {
        Self::Failed { message }
    }
}

#[derive(uniffi::Object)]
pub struct RustCore {
    app: App,
}

#[derive(uniffi::Record)]
pub struct ImportSource {
    pub kind: ImporterKind,
    pub name: String,
    pub extensions: Vec<String>,
    pub outputs: Vec<String>,
}

#[uniffi::export]
pub fn import_sources() -> Vec<ImportSource> {
    use pitpls_importers::model::{InputType, OutputType};
    pitpls_importers::IMPORTERS
        .iter()
        .map(|source| ImportSource {
            kind: match source.kind {
                ImporterKind::T212 => ImporterKind::T212,
                ImporterKind::Revolut => ImporterKind::Revolut,
                ImporterKind::Coinbase => ImporterKind::Coinbase,
            },
            name: source.name.into(),
            extensions: source
                .input
                .iter()
                .map(|input| {
                    match input {
                        InputType::Csv => "csv",
                        InputType::Pdf => "pdf",
                    }
                    .into()
                })
                .collect(),
            outputs: source
                .output
                .iter()
                .map(|output| {
                    match output {
                        OutputType::Dividend => "Dividends",
                        OutputType::Crypto => "Crypto",
                        OutputType::Interest => "Interests",
                    }
                    .into()
                })
                .collect(),
        })
        .collect()
}

#[uniffi::export(async_runtime = "tokio")]
impl RustCore {
    #[uniffi::constructor]
    pub async fn open(database_path: String) -> Result<Arc<Self>, NativeError> {
        let db = Database::open(database_path)
            .await
            .map_err(|error| NativeError::from(error.to_string()))?;
        Ok(Arc::new(Self { app: App::new(db) }))
    }

    pub async fn create_crypto(&self, input: CreateCryptoInput) -> Result<String, NativeError> {
        crypto::create_crypto(&self.app, input)
            .await
            .map_err(Into::into)
    }

    pub async fn update_crypto(&self, input: UpdateCryptoInput) -> Result<(), NativeError> {
        crypto::update_crypto(&self.app, input)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_cryptos(&self, ids: Vec<String>) -> Result<u64, NativeError> {
        crypto::delete_cryptos(&self.app, ids)
            .await
            .map_err(Into::into)
    }

    pub async fn load_cryptos(&self, year: Option<i32>) -> Result<CryptoTaxData, NativeError> {
        crypto::load_cryptos(&self.app, year)
            .await
            .map_err(Into::into)
    }

    pub async fn create_dividend(&self, input: CreateDividendInput) -> Result<String, NativeError> {
        dividend::create_dividend(&self.app, input)
            .await
            .map_err(Into::into)
    }

    pub async fn update_dividend(&self, input: UpdateDividendInput) -> Result<(), NativeError> {
        dividend::update_dividend(&self.app, input)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_dividends(&self, ids: Vec<String>) -> Result<u64, NativeError> {
        dividend::delete_dividends(&self.app, ids)
            .await
            .map_err(Into::into)
    }

    pub async fn load_dividends(&self, year: Option<i32>) -> Result<DividendTaxData, NativeError> {
        dividend::load_dividends(&self.app, year)
            .await
            .map_err(Into::into)
    }

    pub async fn create_interest(&self, input: CreateInterestInput) -> Result<String, NativeError> {
        interest::create_interest(&self.app, input)
            .await
            .map_err(Into::into)
    }

    pub async fn update_interest(&self, input: UpdateInterestInput) -> Result<(), NativeError> {
        interest::update_interest(&self.app, input)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_interests(&self, ids: Vec<String>) -> Result<u64, NativeError> {
        interest::delete_interests(&self.app, ids)
            .await
            .map_err(Into::into)
    }

    pub async fn load_interests(&self, year: Option<i32>) -> Result<InterestTaxData, NativeError> {
        interest::load_interests(&self.app, year)
            .await
            .map_err(Into::into)
    }

    pub async fn run_import(
        &self,
        kind: ImporterKind,
        file: String,
    ) -> Result<ImportResult, NativeError> {
        import::run_import(&self.app, kind, file)
            .await
            .map_err(Into::into)
    }

    pub async fn import_csv(&self, file: String) -> Result<u64, NativeError> {
        rate::import_csv(&self.app, file).await.map_err(Into::into)
    }

    pub async fn import_api(&self, year: i32) -> Result<u64, NativeError> {
        rate::import_api(&self.app, year).await.map_err(Into::into)
    }

    pub async fn reset_rates(&self) -> Result<u64, NativeError> {
        rate::reset_rates(&self.app).await.map_err(Into::into)
    }

    pub async fn list_rates(&self) -> Result<RatesViewModel, NativeError> {
        rate::list_rates(&self.app).await.map_err(Into::into)
    }

    pub async fn load_settings(&self) -> Result<Settings, NativeError> {
        settings::load_settings(&self.app).await.map_err(Into::into)
    }

    pub async fn update_settings(&self, settings: Settings) -> Result<(), NativeError> {
        pitpls_app::use_case::settings::update_settings(&self.app, settings)
            .await
            .map_err(Into::into)
    }

    pub async fn load_tax_summary(&self, year: Option<i32>) -> Result<TaxSummary, NativeError> {
        tax::load_tax_summary(&self.app, year)
            .await
            .map_err(Into::into)
    }

    pub async fn get_warnings(&self, year: Option<i32>) -> Result<Warnings, NativeError> {
        warnings::get_warnings(&self.app, year)
            .await
            .map_err(Into::into)
    }

    pub async fn list_years(&self) -> Result<Vec<i32>, NativeError> {
        year_use_case::list_years(&self.app)
            .await
            .map_err(Into::into)
    }

    pub async fn add_year(&self, year: i32) -> Result<(), NativeError> {
        year_use_case::add_year(&self.app, year)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_year(&self, year: i32) -> Result<u64, NativeError> {
        year_use_case::delete_year(&self.app, year)
            .await
            .map_err(Into::into)
    }
}
