use std::str::FromStr;

use anyhow::Result;
use chrono::NaiveDate;
use pitpls_core::{common::Amount, dividend::Dividend};
use rust_decimal::Decimal;
use sqlx::{Row, SqlitePool};

pub struct DividendRepository {
    db: SqlitePool,
}

impl DividendRepository {
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
            let result = sqlx::query("DELETE FROM dividends WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            rows += result.rows_affected();
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn save(&self, dividends: &[Dividend]) -> Result<u64> {
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for dividend in dividends {
            rows += 1;
            sqlx::query(
                r"
                    INSERT INTO dividends(id, date, ticker, value, value_currency, tax_paid, tax_paid_currency, country, provider)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(dividend.id.to_string())
            .bind(dividend.date.to_string())
            .bind(dividend.ticker.to_string())
            .bind(dividend.value.value.to_string())
            .bind(serde_plain::to_string(&dividend.value.currency)?)
            .bind(dividend.tax_paid.value.to_string())
            .bind(serde_plain::to_string(&dividend.tax_paid.currency)?)
            .bind(serde_plain::to_string(&dividend.country)?)
            .bind(dividend.provider.to_string())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(rows)
    }

    pub async fn update(&self, d: &Dividend) -> Result<u64> {
        let result = sqlx::query(
            r"
                UPDATE dividends
                SET date = ?, ticker = ?, value = ?, value_currency = ?, tax_paid = ?, tax_paid_currency = ?, country = ?, provider = ?
                WHERE id = ?
            ",
        )
        .bind(d.date.to_string())
        .bind(d.ticker.to_string())
        .bind(d.value.value.to_string())
        .bind(serde_plain::to_string(&d.value.currency)?)
        .bind(d.tax_paid.value.to_string())
        .bind(serde_plain::to_string(&d.tax_paid.currency)?)
        .bind(serde_plain::to_string(&d.country)?)
        .bind(d.provider.to_string())
        .bind(d.id.to_string())
        .execute(&self.db)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_by_year(&self, year: Option<i32>) -> Result<Vec<Dividend>> {
        const BASE: &str =
            "SELECT id, date, ticker, value, value_currency, tax_paid, tax_paid_currency, country, provider FROM dividends";
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
                let ticker: String = row.try_get("ticker")?;
                let value: String = row.try_get("value")?;
                let value_currency: String = row.try_get("value_currency")?;
                let tax_paid: String = row.try_get("tax_paid")?;
                let tax_paid_currency: String = row.try_get("tax_paid_currency")?;
                let country: String = row.try_get("country")?;
                let provider: String = row.try_get("provider")?;

                Ok(Dividend {
                    id,
                    date: NaiveDate::parse_from_str(&date, "%Y-%m-%d")?,
                    ticker,
                    value: Amount {
                        value: Decimal::from_str(&value)?,
                        currency: serde_plain::from_str(&value_currency)?,
                    },
                    tax_paid: Amount {
                        value: Decimal::from_str(&tax_paid)?,
                        currency: serde_plain::from_str(&tax_paid_currency)?,
                    },
                    country: serde_plain::from_str(&country)?,
                    provider,
                })
            })
            .collect()
    }
}
