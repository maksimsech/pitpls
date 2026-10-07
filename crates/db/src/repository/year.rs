use sqlx::{Row, SqlitePool};

use super::Result;

/// A year that has records or was added to the `years` table.
pub struct YearRecords {
    pub year: i32,
    pub dividends: u32,
    pub interests: u32,
    pub cryptos: u32,
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
        let rows = sqlx::query(
            r"
                SELECT year,
                       SUM(dividends) AS dividends,
                       SUM(interests) AS interests,
                       SUM(cryptos) AS cryptos,
                       MAX(custom) AS custom
                FROM (
                    SELECT CAST(strftime('%Y', date) AS INTEGER) AS year,
                           COUNT(*) AS dividends, 0 AS interests, 0 AS cryptos, 0 AS custom
                    FROM dividends GROUP BY 1
                    UNION ALL
                    SELECT CAST(strftime('%Y', date) AS INTEGER), 0, COUNT(*), 0, 0
                    FROM interests GROUP BY 1
                    UNION ALL
                    SELECT CAST(strftime('%Y', date) AS INTEGER), 0, 0, COUNT(*), 0
                    FROM cryptos GROUP BY 1
                    UNION ALL
                    SELECT year, 0, 0, 0, 1 FROM years
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

    pub async fn delete(&self, year: i32) -> Result<u64> {
        let result = sqlx::query("DELETE FROM years WHERE year = ?")
            .bind(year)
            .execute(&self.db)
            .await?;
        Ok(result.rows_affected())
    }
}
