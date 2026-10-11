use chrono::NaiveDate;
use pitpls_core::{common::Mic, exchange::Exchange};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};

use super::{RepositoryError, Result};

const COLUMNS: &str = "mic, operating_mic, kind, name, acronym, country, city, status, last_update";

pub struct ExchangeRepository {
    db: SqlitePool,
}

impl ExchangeRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn replace_all(&self, exchanges: &[Exchange]) -> Result<u64> {
        let mut tx = self.db.begin().await?;
        sqlx::query("DELETE FROM exchanges")
            .execute(&mut *tx)
            .await?;

        let insert = format!("INSERT INTO exchanges({COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)");
        let mut rows = 0;
        for exchange in exchanges {
            sqlx::query(&insert)
                .bind(exchange.mic.as_str())
                .bind(exchange.operating_mic.as_str())
                .bind(serde_plain::to_string(&exchange.kind)?)
                .bind(&exchange.name)
                .bind(&exchange.acronym)
                .bind(exchange.country.to_string())
                .bind(&exchange.city)
                .bind(serde_plain::to_string(&exchange.status)?)
                .bind(exchange.last_update)
                .execute(&mut *tx)
                .await?;
            rows += 1;
        }

        tx.commit().await?;

        Ok(rows)
    }

    pub async fn list(&self) -> Result<Vec<Exchange>> {
        sqlx::query(&format!("SELECT {COLUMNS} FROM exchanges ORDER BY mic"))
            .fetch_all(&self.db)
            .await?
            .iter()
            .map(Self::exchange)
            .collect()
    }

    pub async fn get(&self, mic: Mic) -> Result<Option<Exchange>> {
        sqlx::query(&format!("SELECT {COLUMNS} FROM exchanges WHERE mic = ?"))
            .bind(mic.as_str())
            .fetch_optional(&self.db)
            .await?
            .as_ref()
            .map(Self::exchange)
            .transpose()
    }

    fn exchange(row: &SqliteRow) -> Result<Exchange> {
        let kind: String = row.try_get("kind")?;
        let country: String = row.try_get("country")?;
        let status: String = row.try_get("status")?;
        let last_update: Option<NaiveDate> = row.try_get("last_update")?;

        Ok(Exchange {
            mic: Self::mic(row.try_get("mic")?)?,
            operating_mic: Self::mic(row.try_get("operating_mic")?)?,
            kind: serde_plain::from_str(&kind)?,
            name: row.try_get("name")?,
            acronym: row.try_get("acronym")?,
            country: country
                .parse()
                .map_err(|_| RepositoryError::InvalidExchange(format!("country '{country}'")))?,
            city: row.try_get("city")?,
            status: serde_plain::from_str(&status)?,
            last_update,
        })
    }

    fn mic(value: String) -> Result<Mic> {
        value
            .parse()
            .map_err(|_| RepositoryError::InvalidExchange(format!("MIC '{value}'")))
    }
}
