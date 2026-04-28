use anyhow::Result;
use pitpls_core::settings::{DividendRounding, Settings};
use sqlx::{Row, SqlitePool};

const DIVIDEND_ROUNDING_SETTING: &str = "dividend_rounding";

pub struct SettingsRepository {
    db: SqlitePool,
}

impl SettingsRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn load(&self) -> Result<Settings> {
        Ok(Settings {
            dividend_rounding: self.load_dividend_rounding().await?,
        })
    }

    pub async fn save(&self, settings: Settings) -> Result<()> {
        self.save_dividend_rounding(settings.dividend_rounding)
            .await
    }

    pub async fn load_dividend_rounding(&self) -> Result<DividendRounding> {
        let row = sqlx::query("SELECT value FROM settings WHERE name = ?")
            .bind(DIVIDEND_ROUNDING_SETTING)
            .fetch_optional(&self.db)
            .await?;

        match row {
            Some(row) => {
                let value: String = row.try_get("value")?;
                Ok(serde_plain::from_str(&value)?)
            }
            None => Ok(DividendRounding::default()),
        }
    }

    pub async fn save_dividend_rounding(&self, rounding: DividendRounding) -> Result<()> {
        sqlx::query(
            r"
                INSERT INTO settings(name, value)
                VALUES (?, ?)
                ON CONFLICT(name) DO UPDATE SET value = excluded.value
            ",
        )
        .bind(DIVIDEND_ROUNDING_SETTING)
        .bind(serde_plain::to_string(&rounding)?)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
