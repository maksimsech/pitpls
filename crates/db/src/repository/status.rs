use chrono::NaiveDate;
use sqlx::SqlitePool;

use super::{RepositoryError, Result};

pub struct StatusRepository {
    db: SqlitePool,
}

impl StatusRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn has_rates(&self, year: Option<i32>) -> Result<bool> {
        self.table_has_records("rates", year).await
    }

    pub async fn has_financial_records(&self, year: Option<i32>) -> Result<bool> {
        for table in ["cryptos", "dividends", "interests"] {
            if self.table_has_records(table, year).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn table_has_records(&self, table: &str, year: Option<i32>) -> Result<bool> {
        let exists: i64 = match year {
            None => {
                sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM {table}) AS e"))
                    .fetch_one(&self.db)
                    .await?
            }
            Some(year) => {
                let start = NaiveDate::from_ymd_opt(year, 1, 1)
                    .ok_or(RepositoryError::InvalidYear(year))?;
                let end = NaiveDate::from_ymd_opt(year, 12, 31)
                    .ok_or(RepositoryError::InvalidYear(year))?;
                sqlx::query_scalar(&format!(
                    "SELECT EXISTS(SELECT 1 FROM {table} WHERE date BETWEEN ? AND ?) AS e"
                ))
                .bind(start)
                .bind(end)
                .fetch_one(&self.db)
                .await?
            }
        };
        Ok(exists != 0)
    }
}
