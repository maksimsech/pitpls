use std::str::FromStr;

use anyhow::Result;
use chrono::NaiveDate;
use pitpls_core::rate::RateExport;
use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

pub struct RateRepository {
    db: SqlitePool,
}

impl RateRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn reset(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM rates").execute(&self.db).await?;
        Ok(result.rows_affected())
    }

    pub async fn upload(&self, rates: impl Iterator<Item = RateExport>) -> Result<u64> {
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for rate in rates {
            rows += 1;
            sqlx::query("INSERT OR REPLACE INTO rates(date, currency, rate) VALUES (?, ?, ?)")
                .bind(rate.date)
                .bind(serde_plain::to_string(&rate.currency)?)
                .bind(rate.rate.to_string())
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(rows)
    }

    pub async fn load_all(&self) -> Result<Vec<RateExport>> {
        let rows = sqlx::query("SELECT date, currency, rate FROM rates")
            .fetch_all(&self.db)
            .await?;

        rows.into_iter()
            .map(|row| {
                let date: NaiveDate = row.try_get("date")?;
                let currency: String = row.try_get("currency")?;
                let rate: String = row.try_get("rate")?;

                Ok(RateExport {
                    date,
                    currency: serde_plain::from_str(&currency)?,
                    rate: Decimal::from_str(&rate)?,
                })
            })
            .collect()
    }
}
