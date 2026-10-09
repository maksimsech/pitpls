use pitpls_core::settings::Settings;

use crate::App;
use pitpls_db::RepositoryError;

pub async fn load_settings(app: &App) -> Result<Settings, RepositoryError> {
    app.db.settings_repo().load().await
}

pub async fn update_settings(app: &App, settings: Settings) -> Result<(), RepositoryError> {
    app.db.settings_repo().save(settings).await
}
