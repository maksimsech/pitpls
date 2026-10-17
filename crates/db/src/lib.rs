use std::{path::Path, str::FromStr, time::Duration};

use sqlx::{
    SqlitePool,
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use thiserror::Error;

pub use repository::RepositoryError;

use repository::{
    crypto::CryptoRepository, dividend::DividendRepository, exchange::ExchangeRepository,
    instrument::InstrumentRepository, interest::InterestRepository, rate::RateRepository,
    settings::SettingsRepository, stock::StockRepository, year::YearRepository,
};

pub mod repository;
#[cfg(test)]
mod tests;

pub const DB_FILENAME: &str = "pitpls.db";

static MIGRATOR: Migrator = sqlx::migrate!();

// SQLite takes one writer at a time, so more connections only help reads.
const MAX_CONNECTIONS: u32 = 4;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum OpenDatabaseError {
    CreateDirectory(#[source] std::io::Error),
    Connect(#[source] sqlx::Error),
    Migrate(#[source] MigrateError),
}

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

        // WAL lets reads go on during a write.
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new().max_connections(MAX_CONNECTIONS);
        Self::connect(pool, options).await
    }

    // An in-memory database is gone when its last connection closes, so the one connection stays.
    pub async fn open_in_memory() -> Result<Self, OpenDatabaseError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(OpenDatabaseError::Connect)?;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None);
        Self::connect(pool, options).await
    }

    async fn connect(
        pool: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) -> Result<Self, OpenDatabaseError> {
        let pool = pool
            .connect_with(options)
            .await
            .map_err(OpenDatabaseError::Connect)?;
        if let Err(error) = MIGRATOR.run(&pool).await {
            // Closed before returning, so the file can be deleted to start over;
            // Windows refuses to delete an open file.
            pool.close().await;
            return Err(OpenDatabaseError::Migrate(error));
        }

        Ok(Self::new(pool))
    }

    /// Also deletes the journal files SQLite keeps beside the database.
    pub fn delete(path: impl AsRef<Path>) -> std::io::Result<()> {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let mut file = path.as_ref().as_os_str().to_owned();
            file.push(suffix);
            match std::fs::remove_file(file) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        Ok(())
    }

    pub fn crypto_repo(&self) -> CryptoRepository {
        CryptoRepository::new(self.pool.clone())
    }

    pub fn dividend_repo(&self) -> DividendRepository {
        DividendRepository::new(self.pool.clone())
    }

    pub fn exchange_repo(&self) -> ExchangeRepository {
        ExchangeRepository::new(self.pool.clone())
    }

    pub fn instrument_repo(&self) -> InstrumentRepository {
        InstrumentRepository::new(self.pool.clone())
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

    pub fn stock_repo(&self) -> StockRepository {
        StockRepository::new(self.pool.clone())
    }

    pub fn year_repo(&self) -> YearRepository {
        YearRepository::new(self.pool.clone())
    }
}
