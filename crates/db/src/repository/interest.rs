use std::str::FromStr;

use chrono::{DateTime, Datelike, Utc};
use pitpls_core::{
    common::Amount,
    interest::Interest,
    time::{tax_date, year_range},
};
use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use super::{RepositoryError, Result};

pub struct InterestRepository {
    db: SqlitePool,
}

impl InterestRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn delete_by_ids(&self, ids: &[String]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for id in ids {
            let result = sqlx::query("DELETE FROM interests WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            rows += result.rows_affected();
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn insert(&self, interest: &Interest) -> Result<()> {
        let mut tx = self.db.begin().await?;

        sqlx::query(
            r"
                INSERT INTO interests(id, time, value, value_currency, provider)
                VALUES (?, ?, ?, ?, ?)
            ",
        )
        .bind(interest.id.to_string())
        .bind(interest.time)
        .bind(interest.value.value.to_string())
        .bind(serde_plain::to_string(&interest.value.currency)?)
        .bind(interest.provider.to_string())
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
            .bind(tax_date(interest.time).year())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn save(&self, interests: &[Interest]) -> Result<u64> {
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for interest in interests {
            let result = sqlx::query(
                r"
                    INSERT INTO interests(id, time, value, value_currency, provider)
                    VALUES (?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        time = excluded.time,
                        value = excluded.value,
                        value_currency = excluded.value_currency,
                        provider = excluded.provider
                    WHERE interests.provider = excluded.provider
                ",
            )
            .bind(interest.id.to_string())
            .bind(interest.time)
            .bind(interest.value.value.to_string())
            .bind(serde_plain::to_string(&interest.value.currency)?)
            .bind(interest.provider.to_string())
            .execute(&mut *tx)
            .await?;

            let affected = result.rows_affected();
            rows += affected;

            if affected > 0 {
                sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
                    .bind(tax_date(interest.time).year())
                    .execute(&mut *tx)
                    .await?;
            }
        }

        tx.commit().await?;

        Ok(rows)
    }

    pub async fn update(&self, i: &Interest) -> Result<u64> {
        let mut tx = self.db.begin().await?;
        let result = sqlx::query(
            r"
                UPDATE interests
                SET time = ?, value = ?, value_currency = ?, provider = ?
                WHERE id = ?
            ",
        )
        .bind(i.time)
        .bind(i.value.value.to_string())
        .bind(serde_plain::to_string(&i.value.currency)?)
        .bind(i.provider.to_string())
        .bind(i.id.to_string())
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
            .bind(tax_date(i.time).year())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(result.rows_affected())
    }

    pub async fn get_by_year(&self, year: Option<i32>) -> Result<Vec<Interest>> {
        const BASE: &str = "SELECT id, time, value, value_currency, provider FROM interests";
        let rows = match year {
            None => sqlx::query(BASE).fetch_all(&self.db).await?,
            Some(y) => {
                let (start, end) = year_range(y).ok_or(RepositoryError::InvalidYear(y))?;
                sqlx::query(&format!("{BASE} WHERE time >= ? AND time < ?"))
                    .bind(start)
                    .bind(end)
                    .fetch_all(&self.db)
                    .await?
            }
        };

        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                let time: DateTime<Utc> = row.try_get("time")?;
                let value: String = row.try_get("value")?;
                let value_currency: String = row.try_get("value_currency")?;
                let provider: String = row.try_get("provider")?;

                Ok(Interest {
                    id,
                    time,
                    value: Amount {
                        value: Decimal::from_str(&value)?,
                        currency: serde_plain::from_str(&value_currency)?,
                    },
                    provider,
                })
            })
            .collect()
    }
}
