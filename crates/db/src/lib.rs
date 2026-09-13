use std::{path::Path, time::Duration};

use sqlx::{
    SqlitePool,
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use thiserror::Error;

pub use repository::RepositoryError;

use repository::{
    crypto::CryptoRepository, dividend::DividendRepository, interest::InterestRepository,
    rate::RateRepository, settings::SettingsRepository, status::StatusRepository,
    year::YearRepository,
};

pub mod repository;

pub const DB_FILENAME: &str = "pitpls.db";

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Debug, Error)]
pub enum OpenDatabaseError {
    #[error("Failed to create the database directory: {0}")]
    CreateDirectory(#[source] std::io::Error),
    #[error("Failed to open the database: {0}")]
    Connect(#[source] sqlx::Error),
    #[error("Failed to migrate the database: {0}")]
    Migrate(#[source] MigrateError),
}

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn open(path: impl AsRef<Path>) -> Result<Self, OpenDatabaseError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(OpenDatabaseError::CreateDirectory)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(OpenDatabaseError::Connect)?;
        MIGRATOR
            .run(&pool)
            .await
            .map_err(OpenDatabaseError::Migrate)?;

        Ok(Self::new(pool))
    }

    pub fn crypto_repo(&self) -> CryptoRepository {
        CryptoRepository::new(self.pool.clone())
    }

    pub fn dividend_repo(&self) -> DividendRepository {
        DividendRepository::new(self.pool.clone())
    }

    pub fn interest_repo(&self) -> InterestRepository {
        InterestRepository::new(self.pool.clone())
    }

    pub fn rate_repo(&self) -> RateRepository {
        RateRepository::new(self.pool.clone())
    }

    pub fn settings_repo(&self) -> SettingsRepository {
        SettingsRepository::new(self.pool.clone())
    }

    pub fn status_repo(&self) -> StatusRepository {
        StatusRepository::new(self.pool.clone())
    }

    pub fn year_repo(&self) -> YearRepository {
        YearRepository::new(self.pool.clone())
    }
}
