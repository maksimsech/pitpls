use std::sync::LazyLock;

use reqwest::Client;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager as _};
use tauri_plugin_sql::{DbInstances, DbPool};

use crate::repository::crypto::CryptoRepository;
use crate::repository::dividend::DividendRepository;
use crate::repository::interest::InterestRepository;
use crate::repository::migration::DB_URL;
use crate::repository::rate::RateRepository;
use crate::repository::settings::SettingsRepository;
use crate::repository::year::YearRepository;

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

pub fn setup(handle: AppHandle) -> std::result::Result<(), Box<dyn std::error::Error>> {
    tauri::async_runtime::block_on(async move {
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
