use std::fmt::Display;

use pitpls_db::RepositoryError;

pub mod crypto;
pub mod dividend;
pub mod import;
pub mod interest;
pub mod rate;
pub mod settings;
pub mod tax;
pub mod warnings;
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
