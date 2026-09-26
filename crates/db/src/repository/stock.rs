use std::str::FromStr;

use chrono::{Datelike, NaiveDate};
use pitpls_core::{
    common::Amount,
    stock::{Stock, StockAction, StockBuy, StockSell, StockSplit},
};
use rust_decimal::Decimal;
use sqlx::{Row, Sqlite, SqlitePool, sqlite::SqliteArguments, sqlite::SqliteRow};

use super::{RepositoryError, Result};

pub struct StockRepository {
    db: SqlitePool,
}

impl StockRepository {
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
            rows += sqlx::query("DELETE FROM stocks WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn insert(&self, stock: &Stock) -> Result<()> {
        let mut tx = self.db.begin().await?;
        StockValues::from(stock)
            .bind(sqlx::query(
                "INSERT INTO stocks(date, ticker, action, number, price, price_currency, fee, fee_currency, provider, ratio, id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ))
            .bind(&stock.id)
            .execute(&mut *tx)
            .await?;
        record_year(&mut tx, stock.date).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn save(&self, stocks: &[Stock]) -> Result<u64> {
        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for stock in stocks {
            let affected = StockValues::from(stock)
                .bind(sqlx::query(
                    r"
                    INSERT INTO stocks(date, ticker, action, number, price, price_currency, fee, fee_currency, provider, ratio, id)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        date = excluded.date,
                        ticker = excluded.ticker,
                        action = excluded.action,
                        number = excluded.number,
                        price = excluded.price,
                        price_currency = excluded.price_currency,
                        fee = excluded.fee,
                        fee_currency = excluded.fee_currency,
                        provider = excluded.provider,
                        ratio = excluded.ratio
                    WHERE stocks.provider IS excluded.provider
                    ",
                ))
                .bind(&stock.id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            rows += affected;
            if affected > 0 {
                record_year(&mut tx, stock.date).await?;
            }
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn update(&self, stock: &Stock) -> Result<u64> {
        let mut tx = self.db.begin().await?;
        let rows = StockValues::from(stock)
            .bind(sqlx::query(
                r"
                UPDATE stocks SET
                    date = ?, ticker = ?, action = ?, number = ?, price = ?,
                    price_currency = ?, fee = ?, fee_currency = ?, provider = ?, ratio = ?
                WHERE id = ?
                ",
            ))
            .bind(&stock.id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if rows > 0 {
            record_year(&mut tx, stock.date).await?;
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn get_by_year(&self, year: Option<i32>) -> Result<Vec<Stock>> {
        let rows = match year {
            None => {
                sqlx::query("SELECT * FROM stocks ORDER BY date, id")
                    .fetch_all(&self.db)
                    .await?
            }
            Some(year) => {
                let start = NaiveDate::from_ymd_opt(year, 1, 1)
                    .ok_or(RepositoryError::InvalidYear(year))?;
                let end = NaiveDate::from_ymd_opt(year, 12, 31)
                    .ok_or(RepositoryError::InvalidYear(year))?;
                sqlx::query("SELECT * FROM stocks WHERE date BETWEEN ? AND ? ORDER BY date, id")
                    .bind(start)
                    .bind(end)
                    .fetch_all(&self.db)
                    .await?
            }
        };
        rows.into_iter().map(read_stock).collect()
    }

    pub async fn get_through_year(&self, year: i32) -> Result<Vec<Stock>> {
        let end =
            NaiveDate::from_ymd_opt(year, 12, 31).ok_or(RepositoryError::InvalidYear(year))?;
        sqlx::query("SELECT * FROM stocks WHERE date <= ? ORDER BY date, id")
            .bind(end)
            .fetch_all(&self.db)
            .await?
            .into_iter()
            .map(read_stock)
            .collect()
    }
}

async fn record_year(tx: &mut sqlx::Transaction<'_, Sqlite>, date: NaiveDate) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
        .bind(date.year())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

struct StockValues {
    date: NaiveDate,
    ticker: String,
    action: &'static str,
    number: Option<String>,
    price: Option<String>,
    price_currency: Option<String>,
    fee: Option<String>,
    fee_currency: Option<String>,
    provider: Option<String>,
    ratio: Option<String>,
}

impl From<&Stock> for StockValues {
    fn from(stock: &Stock) -> Self {
        let (action, number, price, price_currency, fee, fee_currency, provider, ratio) =
            match &stock.action {
                StockAction::Buy(buy) => (
                    "Buy",
                    Some(buy.number.to_string()),
                    Some(buy.price.value.to_string()),
                    Some(buy.price.currency.to_string()),
                    Some(buy.fee.value.to_string()),
                    Some(buy.fee.currency.to_string()),
                    Some(buy.provider.clone()),
                    None,
                ),
                StockAction::Sell(sell) => (
                    "Sell",
                    Some(sell.number.to_string()),
                    Some(sell.price.value.to_string()),
                    Some(sell.price.currency.to_string()),
                    Some(sell.fee.value.to_string()),
                    Some(sell.fee.currency.to_string()),
                    Some(sell.provider.clone()),
                    None,
                ),
                StockAction::Split(split) => (
                    "Split",
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(split.ratio.to_string()),
                ),
            };
        Self {
            date: stock.date,
            ticker: stock.ticker.clone(),
            action,
            number,
            price,
            price_currency,
            fee,
            fee_currency,
            provider,
            ratio,
        }
    }
}

impl StockValues {
    fn bind<'q>(
        self,
        query: sqlx::query::Query<'q, Sqlite, SqliteArguments<'q>>,
    ) -> sqlx::query::Query<'q, Sqlite, SqliteArguments<'q>> {
        query
            .bind(self.date)
            .bind(self.ticker)
            .bind(self.action)
            .bind(self.number)
            .bind(self.price)
            .bind(self.price_currency)
            .bind(self.fee)
            .bind(self.fee_currency)
            .bind(self.provider)
            .bind(self.ratio)
    }
}

fn read_stock(row: SqliteRow) -> Result<Stock> {
    let id: String = row.try_get("id")?;
    let date: NaiveDate = row.try_get("date")?;
    let ticker: String = row.try_get("ticker")?;
    let action: String = row.try_get("action")?;

    let action = match action.as_str() {
        "Buy" | "Sell" => {
            let number = Decimal::from_str(&required(&row, "number", &id)?)?;
            let price = Amount {
                value: Decimal::from_str(&required(&row, "price", &id)?)?,
                currency: serde_plain::from_str(&required(&row, "price_currency", &id)?)?,
            };
            let fee = Amount {
                value: Decimal::from_str(&required(&row, "fee", &id)?)?,
                currency: serde_plain::from_str(&required(&row, "fee_currency", &id)?)?,
            };
            let provider = required(&row, "provider", &id)?;
            if action == "Buy" {
                StockAction::Buy(StockBuy {
                    number,
                    price,
                    fee,
                    provider,
                })
            } else {
                StockAction::Sell(StockSell {
                    number,
                    price,
                    fee,
                    provider,
                })
            }
        }
        "Split" => StockAction::Split(StockSplit {
            ratio: Decimal::from_str(&required(&row, "ratio", &id)?)?,
        }),
        _ => {
            return Err(RepositoryError::InvalidStockRecord(format!(
                "unknown action '{action}' for ID '{id}'"
            )));
        }
    };

    Ok(Stock {
        id,
        date,
        ticker,
        action,
    })
}

fn required(row: &SqliteRow, field: &str, id: &str) -> Result<String> {
    row.try_get::<Option<String>, _>(field)?.ok_or_else(|| {
        RepositoryError::InvalidStockRecord(format!("missing {field} for ID '{id}'"))
    })
}
