use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use super::Result;

/// The last statement imported successfully.
pub struct LastImport {
    /// The importer's name, such as "Trading 212".
    pub provider: String,
    /// The file's name, without its directory.
    pub file_name: String,
    pub dividends: u32,
    pub interests: u32,
    pub cryptos: u32,
    pub imported_at: DateTime<Utc>,
}

/// One row, replaced by each import.
pub struct LastImportRepository {
    db: SqlitePool,
}

impl LastImportRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn load(&self) -> Result<Option<LastImport>> {
        let row = sqlx::query(
            r"
                SELECT provider, file_name, dividends, interests, cryptos, imported_at
                FROM last_import
                WHERE id = 1
            ",
        )
        .fetch_optional(&self.db)
        .await?;
        row.map(|row| {
            Ok(LastImport {
                provider: row.try_get("provider")?,
                file_name: row.try_get("file_name")?,
                dividends: row.try_get("dividends")?,
                interests: row.try_get("interests")?,
                cryptos: row.try_get("cryptos")?,
                imported_at: row.try_get("imported_at")?,
            })
        })
        .transpose()
    }

    pub async fn save(&self, import: &LastImport) -> Result<()> {
        sqlx::query(
            r"
                INSERT INTO last_import(id, provider, file_name, dividends, interests, cryptos, imported_at)
                VALUES (1, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    provider = excluded.provider,
                    file_name = excluded.file_name,
                    dividends = excluded.dividends,
                    interests = excluded.interests,
                    cryptos = excluded.cryptos,
                    imported_at = excluded.imported_at
            ",
        )
        .bind(&import.provider)
        .bind(&import.file_name)
        .bind(import.dividends)
        .bind(import.interests)
        .bind(import.cryptos)
        .bind(import.imported_at)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}
