use std::error::Error;
use std::sync::LazyLock;

use reqwest::Client;
use sqlx::SqlitePool;
use tauri::async_runtime::block_on;
use tauri::{AppHandle, Manager as _};
use tauri_plugin_sql::{DbInstances, DbPool};

use crate::db::DB_URL;
use crate::repository::{
    crypto::CryptoRepository, dividend::DividendRepository, interest::InterestRepository,
    rate::RateRepository, settings::SettingsRepository, year::YearRepository,
};

pub struct AppState {
    db: SqlitePool,
    api_client: LazyLock<Client>,
}

impl AppState {
    pub fn db_pool(&self) -> &SqlitePool {
        &self.db
    }

    pub fn api_client(&self) -> &Client {
        &self.api_client
    }

    pub fn rate_repo(&self) -> RateRepository {
        RateRepository::new(self.db.clone())
    }

    pub fn crypto_repo(&self) -> CryptoRepository {
        CryptoRepository::new(self.db.clone())
    }

    pub fn dividend_repo(&self) -> DividendRepository {
        DividendRepository::new(self.db.clone())
    }

    pub fn interest_repo(&self) -> InterestRepository {
        InterestRepository::new(self.db.clone())
    }

    pub fn year_repo(&self) -> YearRepository {
        YearRepository::new(self.db.clone())
    }

    pub fn settings_repo(&self) -> SettingsRepository {
        SettingsRepository::new(self.db.clone())
    }
}

pub fn setup(handle: AppHandle) -> Result<(), Box<dyn Error>> {
    block_on(async move {
        let instances = handle.state::<DbInstances>();
        let map = instances.0.read().await;
        let DbPool::Sqlite(pool) = map.get(DB_URL).ok_or("db not loaded")?;

        handle.manage(AppState {
            db: pool.clone(),
            api_client: LazyLock::new(Client::new),
        });
        Ok(())
    })
}
