use super::{error_message, validate_year};
use crate::App;

pub async fn list_years(app: &App) -> Result<Vec<i32>, String> {
    app.db.year_repo().list().await.map_err(error_message)
}

pub async fn add_year(app: &App, year: i32) -> Result<(), String> {
    validate_year(year)?;
    app.db.year_repo().add(year).await.map_err(error_message)
}

pub async fn delete_year(app: &App, year: i32) -> Result<u64, String> {
    validate_year(year)?;
    app.db.year_repo().delete(year).await.map_err(error_message)
}
