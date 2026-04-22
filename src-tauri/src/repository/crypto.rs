use std::str::FromStr;

use anyhow::Result;
use chrono::NaiveDate;
use pitpls_core::{common::Amount, crypto::Crypto};
use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

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

    pub async fn save(&self, cryptos: &[Crypto]) -> Result<u64> {
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for crypto in cryptos {
            rows += 1;
            sqlx::query(
                r"
                    INSERT INTO cryptos(id, date, value, value_currency, fee, fee_currency, action, provider)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(crypto.id.to_string())
            .bind(crypto.date.to_string())
            .bind(crypto.value.value.to_string())
            .bind(serde_plain::to_string(&crypto.value.currency)?)
            .bind(crypto.fee.value.to_string())
            .bind(serde_plain::to_string(&crypto.fee.currency)?)
            .bind(serde_plain::to_string(&crypto.action)?)
            .bind(crypto.provider.to_string())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(rows)
    }

    pub async fn update(&self, c: &Crypto) -> Result<u64> {
        let result = sqlx::query(
            r"
                UPDATE cryptos
                SET date = ?, value = ?, value_currency = ?, fee = ?, fee_currency = ?, action = ?, provider = ?
                WHERE id = ?
            ",
        )
        .bind(c.date.to_string())
        .bind(c.value.value.to_string())
        .bind(serde_plain::to_string(&c.value.currency)?)
        .bind(c.fee.value.to_string())
        .bind(serde_plain::to_string(&c.fee.currency)?)
        .bind(serde_plain::to_string(&c.action)?)
        .bind(c.provider.to_string())
        .bind(c.id.to_string())
        .execute(&self.db)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_by_year(&self, year: Option<i32>) -> Result<Vec<Crypto>> {
        const BASE: &str =
            "SELECT id, date, value, value_currency, fee, fee_currency, action, provider FROM cryptos";
        let rows = match year {
            None => sqlx::query(BASE).fetch_all(&self.db).await?,
            Some(y) => sqlx::query(&format!("{BASE} WHERE date BETWEEN ? AND ?"))
                .bind(format!("{y:04}-01-01"))
                .bind(format!("{y:04}-12-31"))
                .fetch_all(&self.db)
                .await?,
        };

        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                let date: String = row.try_get("date")?;
                let value: String = row.try_get("value")?;
                let value_currency: String = row.try_get("value_currency")?;
                let fee: String = row.try_get("fee")?;
                let fee_currency: String = row.try_get("fee_currency")?;
                let action: String = row.try_get("action")?;
                let provider: String = row.try_get("provider")?;

                Ok(Crypto {
                    id,
                    date: NaiveDate::parse_from_str(&date, "%Y-%m-%d")?,
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
