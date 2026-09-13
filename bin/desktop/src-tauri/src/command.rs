use pitpls_app::{
    App,
    use_case::{
        crypto::{self, CreateCryptoInput, UpdateCryptoInput},
        dividend::{self, CreateDividendInput, UpdateDividendInput},
        import::{self, ImportResult},
        interest::{self, CreateInterestInput, UpdateInterestInput},
        rate::{self, RatesViewModel},
        settings, tax,
        warnings::{self, Warnings},
        year as year_use_case,
    },
};
use pitpls_core::{
    crypto::CryptoTaxData, dividend::DividendTaxData, interest::InterestTaxData,
    settings::Settings, summary::TaxSummary,
};
use pitpls_importers::model::ImporterKind;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn create_crypto(
    app: State<'_, App>,
    input: CreateCryptoInput,
) -> Result<String, String> {
    crypto::create_crypto(&app, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_crypto(app: State<'_, App>, input: UpdateCryptoInput) -> Result<(), String> {
    crypto::update_crypto(&app, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_cryptos(app: State<'_, App>, ids: Vec<String>) -> Result<u64, String> {
    crypto::delete_cryptos(&app, ids).await
}

#[tauri::command]
#[specta::specta]
pub async fn load_cryptos(app: State<'_, App>, year: Option<i32>) -> Result<CryptoTaxData, String> {
    crypto::load_cryptos(&app, year).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_dividend(
    app: State<'_, App>,
    input: CreateDividendInput,
) -> Result<String, String> {
    dividend::create_dividend(&app, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_dividend(
    app: State<'_, App>,
    input: UpdateDividendInput,
) -> Result<(), String> {
    dividend::update_dividend(&app, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_dividends(app: State<'_, App>, ids: Vec<String>) -> Result<u64, String> {
    dividend::delete_dividends(&app, ids).await
}

#[tauri::command]
#[specta::specta]
pub async fn load_dividends(
    app: State<'_, App>,
    year: Option<i32>,
) -> Result<DividendTaxData, String> {
    dividend::load_dividends(&app, year).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_interest(
    app: State<'_, App>,
    input: CreateInterestInput,
) -> Result<String, String> {
    interest::create_interest(&app, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_interest(
    app: State<'_, App>,
    input: UpdateInterestInput,
) -> Result<(), String> {
    interest::update_interest(&app, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_interests(app: State<'_, App>, ids: Vec<String>) -> Result<u64, String> {
    interest::delete_interests(&app, ids).await
}

#[tauri::command]
#[specta::specta]
pub async fn load_interests(
    app: State<'_, App>,
    year: Option<i32>,
) -> Result<InterestTaxData, String> {
    interest::load_interests(&app, year).await
}

#[tauri::command]
#[specta::specta]
pub async fn run_import(
    app: State<'_, App>,
    kind: ImporterKind,
    file: String,
) -> Result<ImportResult, String> {
    import::run_import(&app, kind, file).await
}

#[tauri::command]
#[specta::specta]
pub async fn import_csv(app: State<'_, App>, file: String) -> Result<u64, String> {
    rate::import_csv(&app, file).await
}

#[tauri::command]
#[specta::specta]
pub async fn import_api(app: State<'_, App>, year: i32) -> Result<u64, String> {
    rate::import_api(&app, year).await
}

#[tauri::command]
#[specta::specta]
pub async fn reset_rates(app: State<'_, App>) -> Result<u64, String> {
    rate::reset_rates(&app).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_rates(app: State<'_, App>) -> Result<RatesViewModel, String> {
    rate::list_rates(&app).await
}

#[tauri::command]
#[specta::specta]
pub async fn load_settings(app: State<'_, App>) -> Result<Settings, String> {
    settings::load_settings(&app).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_settings(app: State<'_, App>, settings: Settings) -> Result<(), String> {
    pitpls_app::use_case::settings::update_settings(&app, settings).await
}

#[tauri::command]
#[specta::specta]
pub async fn load_tax_summary(
    app: State<'_, App>,
    year: Option<i32>,
) -> Result<TaxSummary, String> {
    tax::load_tax_summary(&app, year).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_warnings(app: State<'_, App>, year: Option<i32>) -> Result<Warnings, String> {
    warnings::get_warnings(&app, year).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_years(app: State<'_, App>) -> Result<Vec<i32>, String> {
    year_use_case::list_years(&app).await
}

#[tauri::command]
#[specta::specta]
pub async fn add_year(app: State<'_, App>, year: i32) -> Result<(), String> {
    year_use_case::add_year(&app, year).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_year(app: State<'_, App>, year: i32) -> Result<u64, String> {
    year_use_case::delete_year(&app, year).await
}
