//! Async adapter between GPUI window state and the existing application layer.
//!
//! Domain and database work runs on an owned Tokio runtime so the window module
//! only coordinates tasks and presentation state.

use std::{future::Future, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result, anyhow};
use directories::ProjectDirs;
use pitpls_app::{
    App as DomainApp,
    use_case::{
        crypto::{
            CreateCryptoInput, UpdateCryptoInput, create_crypto, delete_cryptos, load_cryptos,
            update_crypto,
        },
        rate::{RatesViewModel, import_api, import_csv, list_rates, reset_rates},
    },
};
use pitpls_core::crypto::CryptoTaxData;
use pitpls_db::{DB_FILENAME, Database};
use tokio::{runtime::Runtime, task::JoinHandle};

pub struct Services {
    app: Arc<DomainApp>,
    runtime: Runtime,
}

impl Services {
    pub fn open() -> Result<Arc<Self>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("pitpls-service")
            .build()
            .context("failed to create the service runtime")?;
        let db_path = database_path()?;
        let database = runtime
            .block_on(Database::open(&db_path))
            .with_context(|| format!("failed to open {}", db_path.display()))?;

        Ok(Arc::new(Self {
            app: Arc::new(DomainApp::new(database)),
            runtime,
        }))
    }

    fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> JoinHandle<T>
    where
        T: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    pub fn load_cryptos(&self, year: Option<i32>) -> JoinHandle<Result<CryptoTaxData, String>> {
        let app = self.app.clone();
        self.spawn(async move { load_cryptos(&app, year).await })
    }

    pub fn create_crypto(&self, input: CreateCryptoInput) -> JoinHandle<Result<(), String>> {
        let app = self.app.clone();
        self.spawn(async move { create_crypto(&app, input).await.map(|_| ()) })
    }

    pub fn update_crypto(&self, input: UpdateCryptoInput) -> JoinHandle<Result<(), String>> {
        let app = self.app.clone();
        self.spawn(async move { update_crypto(&app, input).await })
    }

    pub fn delete_cryptos(&self, ids: Vec<String>) -> JoinHandle<Result<(), String>> {
        let app = self.app.clone();
        self.spawn(async move { delete_cryptos(&app, ids).await.map(|_| ()) })
    }

    pub fn list_rates(&self) -> JoinHandle<Result<RatesViewModel, String>> {
        let app = self.app.clone();
        self.spawn(async move { list_rates(&app).await })
    }

    pub fn import_rate_csv(&self, path: String) -> JoinHandle<Result<(), String>> {
        let app = self.app.clone();
        self.spawn(async move { import_csv(&app, path).await.map(|_| ()) })
    }

    pub fn import_nbp(&self, year: i32) -> JoinHandle<Result<(), String>> {
        let app = self.app.clone();
        self.spawn(async move { import_api(&app, year).await.map(|_| ()) })
    }

    pub fn reset_rates(&self) -> JoinHandle<Result<(), String>> {
        let app = self.app.clone();
        self.spawn(async move { reset_rates(&app).await.map(|_| ()) })
    }
}

fn database_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "mngapp", "pitpls-gpui").ok_or_else(|| {
        anyhow!("the operating system did not provide an application data directory")
    })?;
    Ok(project_dirs.data_dir().join(DB_FILENAME))
}
