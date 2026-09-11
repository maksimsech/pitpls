use pitpls_core::settings::Settings;

use super::error_message;
use crate::App;

pub async fn load_settings(app: &App) -> Result<Settings, String> {
    app.db.settings_repo().load().await.map_err(error_message)
}

pub async fn update_settings(app: &App, settings: Settings) -> Result<(), String> {
    app.db
        .settings_repo()
        .save(settings)
        .await
        .map_err(error_message)
}
