use std::fmt::Display;

use crate::repository::RepositoryError;

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
