use std::str::FromStr;

use chrono::{DateTime, Datelike, Utc};
use pitpls_core::{
    common::Amount,
    crypto::Crypto,
    time::{tax_date, year_range},
};
use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

use super::{RepositoryError, Result};

pub struct CryptoRepository {
    db: SqlitePool,
}

impl CryptoRepository {
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
            let result = sqlx::query("DELETE FROM cryptos WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            rows += result.rows_affected();
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn insert(&self, crypto: &Crypto) -> Result<()> {
        let mut tx = self.db.begin().await?;

        sqlx::query(
            r"
                INSERT INTO cryptos(id, time, value, value_currency, fee, fee_currency, action, provider)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(crypto.id.to_string())
        .bind(crypto.time)
        .bind(crypto.value.value.to_string())
        .bind(serde_plain::to_string(&crypto.value.currency)?)
        .bind(crypto.fee.value.to_string())
        .bind(serde_plain::to_string(&crypto.fee.currency)?)
        .bind(serde_plain::to_string(&crypto.action)?)
        .bind(crypto.provider.to_string())
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
            .bind(tax_date(crypto.time).year())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn save(&self, cryptos: &[Crypto]) -> Result<u64> {
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for crypto in cryptos {
            let result = sqlx::query(
                r"
                    INSERT INTO cryptos(id, time, value, value_currency, fee, fee_currency, action, provider)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        time = excluded.time,
                        value = excluded.value,
                        value_currency = excluded.value_currency,
                        fee = excluded.fee,
                        fee_currency = excluded.fee_currency,
                        action = excluded.action,
                        provider = excluded.provider
                    WHERE cryptos.provider = excluded.provider
                ",
            )
            .bind(crypto.id.to_string())
            .bind(crypto.time)
            .bind(crypto.value.value.to_string())
            .bind(serde_plain::to_string(&crypto.value.currency)?)
            .bind(crypto.fee.value.to_string())
            .bind(serde_plain::to_string(&crypto.fee.currency)?)
            .bind(serde_plain::to_string(&crypto.action)?)
            .bind(crypto.provider.to_string())
            .execute(&mut *tx)
            .await?;

            let affected = result.rows_affected();
            rows += affected;

            if affected > 0 {
                sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
                    .bind(tax_date(crypto.time).year())
                    .execute(&mut *tx)
                    .await?;
            }
        }

        tx.commit().await?;

        Ok(rows)
    }

    pub async fn update(&self, c: &Crypto) -> Result<u64> {
        let mut tx = self.db.begin().await?;
        let result = sqlx::query(
            r"
                UPDATE cryptos
                SET time = ?, value = ?, value_currency = ?, fee = ?, fee_currency = ?, action = ?, provider = ?
                WHERE id = ?
            ",
        )
        .bind(c.time)
        .bind(c.value.value.to_string())
        .bind(serde_plain::to_string(&c.value.currency)?)
        .bind(c.fee.value.to_string())
        .bind(serde_plain::to_string(&c.fee.currency)?)
        .bind(serde_plain::to_string(&c.action)?)
        .bind(c.provider.to_string())
        .bind(c.id.to_string())
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
            .bind(tax_date(c.time).year())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(result.rows_affected())
    }

    pub async fn get_by_year(&self, year: Option<i32>) -> Result<Vec<Crypto>> {
        const BASE: &str = "SELECT id, time, value, value_currency, fee, fee_currency, action, provider FROM cryptos";
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
                let fee: String = row.try_get("fee")?;
                let fee_currency: String = row.try_get("fee_currency")?;
                let action: String = row.try_get("action")?;
                let provider: String = row.try_get("provider")?;

                Ok(Crypto {
                    id,
                    time,
                    value: Amount {
                        value: Decimal::from_str(&value)?,
                        currency: serde_plain::from_str(&value_currency)?,
                    },
                    fee: Amount {
                        value: Decimal::from_str(&fee)?,
                        currency: serde_plain::from_str(&fee_currency)?,
                    },
                    action: serde_plain::from_str(&action)?,
                    provider,
                })
            })
            .collect()
    }
}
