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

mod validation;

pub use validation::{AmountField, Error as ValidationError};

async fn rates_for(app: &App, date: NaiveDate) -> Result<NbpRateProvider, RepositoryError> {
    let (first, last) = NbpRateProvider::lookup_window(date);
    let rates = app.db.rate_repo().load_range(first, last).await?;
    Ok(NbpRateProvider::new(rates))
}
