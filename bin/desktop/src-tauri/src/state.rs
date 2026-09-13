use std::error::Error;

use pitpls_app::App;
use pitpls_db::{DB_FILENAME, Database};
use tauri::async_runtime::block_on;
use tauri::{AppHandle, Manager as _};

pub fn setup(handle: AppHandle) -> Result<(), Box<dyn Error>> {
    let db_path = handle.path().app_data_dir()?.join(DB_FILENAME);
    let database = block_on(Database::open(db_path))?;
    handle.manage(App::new(database));
    Ok(())
}
