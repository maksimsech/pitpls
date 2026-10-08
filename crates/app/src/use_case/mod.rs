use std::fmt::Display;

use chrono::NaiveDate;
use pitpls_core::rate::NbpRateProvider;
use pitpls_db::RepositoryError;

use crate::App;

pub mod crypto;
pub mod dividend;
pub mod import;
pub mod interest;
pub mod rate;
pub mod settings;
pub mod tax;
pub mod year;

fn error_message(error: impl Display) -> String {
    error.to_string()
}

fn duplicate_id_error(error: RepositoryError, entity: &str, id: &str) -> String {
    if error.is_unique_violation() {
        format!("{entity} with ID '{id}' already exists")
    } else {
        error_message(error)
    }
}

fn validate_year(year: i32) -> Result<(), String> {
    if (1900..=2100).contains(&year) {
        Ok(())
    } else {
        Err(format!("Year {year} out of range (1900-2100)"))
    }
}

fn validate_optional_year(year: Option<i32>) -> Result<(), String> {
    year.map_or(Ok(()), validate_year)
}

/// A converter for one record dated `date`, for previews: it holds only the
/// rates a conversion on that date can read, and converts exactly as one
/// built from every rate would.
async fn rates_for(app: &App, date: NaiveDate) -> Result<NbpRateProvider, String> {
    let (first, last) = NbpRateProvider::lookup_window(date);
    let rates = app
        .db
        .rate_repo()
        .load_range(first, last)
        .await
        .map_err(error_message)?;
    Ok(NbpRateProvider::new(rates))
}
