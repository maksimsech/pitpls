use anyhow::Result;
use sqlx::{Row, SqlitePool};

pub struct YearRepository {
    db: SqlitePool,
}

impl YearRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn list(&self) -> Result<Vec<i32>> {
        let rows = sqlx::query("SELECT year FROM years ORDER BY year DESC")
            .fetch_all(&self.db)
            .await?;
        rows.into_iter()
            .map(|row| Ok(row.try_get::<i32, _>("year")?))
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
