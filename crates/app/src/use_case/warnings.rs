use serde::Serialize;
use specta::Type;

use super::{error_message, validate_optional_year};
use crate::App;

#[derive(Serialize, Type)]
pub struct Warnings {
    pub rates_empty: bool,
    pub has_records_in_year: bool,
}

pub async fn get_warnings(app: &App, year: Option<i32>) -> Result<Warnings, String> {
    validate_optional_year(year)?;
    let status = app.db.status_repo();
    let has_rates = status.has_rates(year).await.map_err(error_message)?;

    let has_records = status
        .has_financial_records(year)
        .await
        .map_err(error_message)?;

    Ok(Warnings {
        rates_empty: !has_rates,
        has_records_in_year: has_records,
    })
}
