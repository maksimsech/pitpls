use std::str::FromStr;

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use pitpls_core::{common::Amount, interest::Interest};
use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

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

    pub async fn save(&self, interests: &[Interest]) -> Result<u64> {
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for interest in interests {
            rows += 1;
            sqlx::query(
                r"
                    INSERT INTO interests(id, date, value, value_currency, provider)
                    VALUES (?, ?, ?, ?, ?)
                ",
            )
            .bind(interest.id.to_string())
            .bind(interest.date)
            .bind(interest.value.value.to_string())
            .bind(serde_plain::to_string(&interest.value.currency)?)
            .bind(interest.provider.to_string())
            .execute(&mut *tx)
            .await?;

            sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
                .bind(interest.date.year())
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(rows)
    }

    pub async fn update(&self, i: &Interest) -> Result<u64> {
        let mut tx = self.db.begin().await?;
        let result = sqlx::query(
            r"
                UPDATE interests
                SET date = ?, value = ?, value_currency = ?, provider = ?
                WHERE id = ?
            ",
        )
        .bind(i.date)
        .bind(i.value.value.to_string())
        .bind(serde_plain::to_string(&i.value.currency)?)
        .bind(i.provider.to_string())
        .bind(i.id.to_string())
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
            .bind(i.date.year())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(result.rows_affected())
    }

    pub async fn get_by_year(&self, year: Option<i32>) -> Result<Vec<Interest>> {
        const BASE: &str = "SELECT id, date, value, value_currency, provider FROM interests";
        let rows = match year {
            None => sqlx::query(BASE).fetch_all(&self.db).await?,
            Some(y) => {
                sqlx::query(&format!("{BASE} WHERE date BETWEEN ? AND ?"))
                    .bind(NaiveDate::from_ymd_opt(y, 1, 1).unwrap())
                    .bind(NaiveDate::from_ymd_opt(y, 12, 31).unwrap())
                    .fetch_all(&self.db)
                    .await?
            }
        };

        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                let date: NaiveDate = row.try_get("date")?;
                let value: String = row.try_get("value")?;
                let value_currency: String = row.try_get("value_currency")?;
                let provider: String = row.try_get("provider")?;

                Ok(Interest {
                    id,
                    date,
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
