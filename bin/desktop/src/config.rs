use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct Config {
    pub database: PathBuf,
    pub preferences: PathBuf,
}

impl Config {
    pub fn from_args() -> Result<Option<Self>, String> {
        let mut database = None;
        let mut args = std::env::args_os().skip(1);
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--database") => {
                    database = Some(PathBuf::from(
                        args.next().ok_or("--database requires a path")?,
                    ));
                }
                Some("--help" | "-h") => {
                    println!(
                        "{} [--database PATH]\n\nDefaults to the pitpls database in the application data directory.",
                        crate::APP_NAME
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown argument: {}", arg.to_string_lossy())),
            }
        }
        let database = match database {
            Some(path) => path,
            None => dirs::data_dir()
                .ok_or("Could not locate the application data directory")?
                .join("com.mngapp.pitpls")
                .join(pitpls_db::DB_FILENAME),
        };
        let preferences = database.with_extension("gpui.json");
        Ok(Some(Self {
            database,
            preferences,
        }))
    }
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
pub struct Preferences {
    pub year: Option<i32>,
    pub dark: Option<bool>,
    #[serde(default)]
    pub sidebar_collapsed: bool,
}

pub async fn read_preferences(path: &std::path::Path) -> Result<Preferences, String> {
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("Could not read preferences: {e}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
        Err(e) => Err(format!("Could not read preferences: {e}")),
    }
}

pub async fn save_preferences(path: PathBuf, preferences: Preferences) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&preferences).map_err(|e| e.to_string())?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| format!("Could not save preferences: {e}"))
}
