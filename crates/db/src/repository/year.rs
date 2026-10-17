use pitpls_core::time::year_range;
use sqlx::{Row, SqlitePool};

use super::{RepositoryError, Result};

/// A year that has records or was added to the `years` table.
pub struct YearRecords {
    pub year: i32,
    pub dividends: u32,
    pub interests: u32,
    pub cryptos: u32,
    pub stocks: u32,
    /// Whether the year is in the `years` table.
    pub custom: bool,
}

pub struct YearRepository {
    db: SqlitePool,
}

impl YearRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Years from the record dates merged with the `years` table, newest
    /// first, with the number of records in each.
    pub async fn list_with_records(&self) -> Result<Vec<YearRecords>> {
        // Poland is on UTC+1 at every New Year, so one hour added to the UTC
        // time gives the Polish year.
        let rows = sqlx::query(
            r"
                SELECT year,
                       SUM(dividends) AS dividends,
                       SUM(interests) AS interests,
                       SUM(cryptos) AS cryptos,
                       SUM(stocks) AS stocks,
                       MAX(custom) AS custom
                FROM (
                    SELECT CAST(strftime('%Y', time, '+1 hour') AS INTEGER) AS year,
                           COUNT(*) AS dividends, 0 AS interests, 0 AS cryptos, 0 AS stocks,
                           0 AS custom
                    FROM dividends GROUP BY 1
                    UNION ALL
                    SELECT CAST(strftime('%Y', time, '+1 hour') AS INTEGER), 0, COUNT(*), 0, 0, 0
                    FROM interests GROUP BY 1
                    UNION ALL
                    SELECT CAST(strftime('%Y', time, '+1 hour') AS INTEGER), 0, 0, COUNT(*), 0, 0
                    FROM cryptos GROUP BY 1
                    UNION ALL
                    SELECT CAST(strftime('%Y', time, '+1 hour') AS INTEGER), 0, 0, 0, COUNT(*), 0
                    FROM stocks GROUP BY 1
                    UNION ALL
                    SELECT year, 0, 0, 0, 0, 1 FROM years
                )
                WHERE year IS NOT NULL
                GROUP BY year
                ORDER BY year DESC
            ",
        )
        .fetch_all(&self.db)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(YearRecords {
                    year: row.try_get("year")?,
                    dividends: row.try_get("dividends")?,
                    interests: row.try_get("interests")?,
                    cryptos: row.try_get("cryptos")?,
                    stocks: row.try_get("stocks")?,
                    custom: row.try_get("custom")?,
                })
            })
            .collect()
    }

    pub async fn add(&self, year: i32) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
            .bind(year)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    // `settled_stock_ids` are sales traded in an earlier year that settle in this one.
    pub async fn delete(&self, year: Option<i32>, settled_stock_ids: &[String]) -> Result<u64> {
        let range = year
            .map(|year| year_range(year).ok_or(RepositoryError::InvalidYear(year)))
            .transpose()?;
        let mut tx = self.db.begin().await?;
        let mut records = 0;
        for table in ["dividends", "interests", "cryptos", "stocks"] {
            let result = match range {
                Some((start, end)) => {
                    sqlx::query(&format!("DELETE FROM {table} WHERE time >= ? AND time < ?"))
                        .bind(start)
                        .bind(end)
                        .execute(&mut *tx)
                        .await?
                }
                None => {
                    sqlx::query(&format!("DELETE FROM {table}"))
                        .execute(&mut *tx)
                        .await?
                }
            };
            records += result.rows_affected();
        }
        for id in settled_stock_ids {
            records += sqlx::query("DELETE FROM stocks WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
        }
        match year {
            Some(year) => {
                sqlx::query("DELETE FROM years WHERE year = ?")
                    .bind(year)
                    .execute(&mut *tx)
                    .await?;
            }
            None => {
                sqlx::query("DELETE FROM years").execute(&mut *tx).await?;
                sqlx::query("DELETE FROM stock_instruments")
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(records)
    }
}
