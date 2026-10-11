use std::str::FromStr;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use pitpls_core::{
    common::Amount,
    stock::{Stock, StockAction, StockBuy, StockSell, StockSplit},
    time::{tax_date, year_range},
};
use rust_decimal::Decimal;
use sqlx::{Row, Sqlite, SqlitePool, sqlite::SqliteArguments, sqlite::SqliteRow};

use super::{RepositoryError, Result};

const COLUMNS: &str = "time, ticker, action, number, price, price_currency, fee, fee_currency, provider, settlement_date, tax_paid, tax_paid_currency, split_before, split_after, id";

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
            .bind(sqlx::query(&format!(
                "INSERT INTO stocks({COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )))
            .bind(&stock.id)
            .execute(&mut *tx)
            .await?;
        record_year(&mut tx, stock.time).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn save(&self, stocks: &[Stock]) -> Result<u64> {
        let query = format!(
            r"
            INSERT INTO stocks({COLUMNS})
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                time = excluded.time,
                ticker = excluded.ticker,
                action = excluded.action,
                number = excluded.number,
                price = excluded.price,
                price_currency = excluded.price_currency,
                fee = excluded.fee,
                fee_currency = excluded.fee_currency,
                provider = excluded.provider,
                settlement_date = excluded.settlement_date,
                tax_paid = excluded.tax_paid,
                tax_paid_currency = excluded.tax_paid_currency,
                split_before = excluded.split_before,
                split_after = excluded.split_after
            WHERE stocks.provider IS excluded.provider
            "
        );

        let mut rows = 0;
        let mut tx = self.db.begin().await?;
        for stock in stocks {
            let affected = StockValues::from(stock)
                .bind(sqlx::query(&query))
                .bind(&stock.id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            rows += affected;
            if affected > 0 {
                record_year(&mut tx, stock.time).await?;
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
                    time = ?, ticker = ?, action = ?, number = ?, price = ?,
                    price_currency = ?, fee = ?, fee_currency = ?, provider = ?,
                    settlement_date = ?, tax_paid = ?, tax_paid_currency = ?,
                    split_before = ?, split_after = ?
                WHERE id = ?
                ",
            ))
            .bind(&stock.id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if rows > 0 {
            record_year(&mut tx, stock.time).await?;
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn get_by_year(&self, year: Option<i32>) -> Result<Vec<Stock>> {
        let rows = match year {
            None => {
                sqlx::query("SELECT * FROM stocks ORDER BY time, id")
                    .fetch_all(&self.db)
                    .await?
            }
            Some(year) => {
                let (start, end) = year_range(year).ok_or(RepositoryError::InvalidYear(year))?;
                sqlx::query("SELECT * FROM stocks WHERE time >= ? AND time < ? ORDER BY time, id")
                    .bind(start)
                    .bind(end)
                    .fetch_all(&self.db)
                    .await?
            }
        };
        rows.into_iter().map(read_stock).collect()
    }

    pub async fn get_through_year(&self, year: i32) -> Result<Vec<Stock>> {
        let (_, end) = year_range(year).ok_or(RepositoryError::InvalidYear(year))?;
        sqlx::query("SELECT * FROM stocks WHERE time < ? ORDER BY time, id")
            .bind(end)
            .fetch_all(&self.db)
            .await?
            .into_iter()
            .map(read_stock)
            .collect()
    }
}

async fn record_year(tx: &mut sqlx::Transaction<'_, Sqlite>, time: DateTime<Utc>) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO years(year) VALUES (?)")
        .bind(tax_date(time).year())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[derive(Default)]
struct StockValues {
    time: DateTime<Utc>,
    ticker: String,
    action: &'static str,
    number: Option<String>,
    price: Option<String>,
    price_currency: Option<String>,
    fee: Option<String>,
    fee_currency: Option<String>,
    provider: Option<String>,
    settlement_date: Option<NaiveDate>,
    tax_paid: Option<String>,
    tax_paid_currency: Option<String>,
    split_before: Option<u32>,
    split_after: Option<u32>,
}

impl From<&Stock> for StockValues {
    fn from(stock: &Stock) -> Self {
        let values = Self {
            time: stock.time,
            ticker: stock.ticker.clone(),
            ..Self::default()
        };
        match &stock.action {
            StockAction::Buy(buy) => Self {
                action: "Buy",
                ..values.with_trade(
                    buy.number,
                    &buy.price,
                    &buy.fee,
                    &buy.provider,
                    buy.settlement_date,
                )
            },
            StockAction::Sell(sell) => Self {
                action: "Sell",
                tax_paid: sell.tax_paid.as_ref().map(|tax| tax.value.to_string()),
                tax_paid_currency: sell.tax_paid.as_ref().map(|tax| tax.currency.to_string()),
                ..values.with_trade(
                    sell.number,
                    &sell.price,
                    &sell.fee,
                    &sell.provider,
                    sell.settlement_date,
                )
            },
            StockAction::Split(split) => Self {
                action: "Split",
                split_before: Some(split.before),
                split_after: Some(split.after),
                ..values
            },
        }
    }
}

impl StockValues {
    fn with_trade(
        self,
        number: Decimal,
        price: &Amount,
        fee: &Amount,
        provider: &str,
        settlement_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            number: Some(number.to_string()),
            price: Some(price.value.to_string()),
            price_currency: Some(price.currency.to_string()),
            fee: Some(fee.value.to_string()),
            fee_currency: Some(fee.currency.to_string()),
            provider: Some(provider.to_owned()),
            settlement_date,
            ..self
        }
    }

    fn bind<'q>(
        self,
        query: sqlx::query::Query<'q, Sqlite, SqliteArguments<'q>>,
    ) -> sqlx::query::Query<'q, Sqlite, SqliteArguments<'q>> {
        query
            .bind(self.time)
            .bind(self.ticker)
            .bind(self.action)
            .bind(self.number)
            .bind(self.price)
            .bind(self.price_currency)
            .bind(self.fee)
            .bind(self.fee_currency)
            .bind(self.provider)
            .bind(self.settlement_date)
            .bind(self.tax_paid)
            .bind(self.tax_paid_currency)
            .bind(self.split_before)
            .bind(self.split_after)
    }
}

fn read_stock(row: SqliteRow) -> Result<Stock> {
    let id: String = row.try_get("id")?;
    let time: DateTime<Utc> = row.try_get("time")?;
    let ticker: String = row.try_get("ticker")?;
    let action: String = row.try_get("action")?;

    let action = match action.as_str() {
        "Buy" => StockAction::Buy(StockBuy {
            number: Decimal::from_str(&required::<String>(&row, "number", &id)?)?,
            price: amount(&row, "price", &id)?,
            fee: amount(&row, "fee", &id)?,
            provider: required(&row, "provider", &id)?,
            settlement_date: row.try_get("settlement_date")?,
        }),
        "Sell" => StockAction::Sell(StockSell {
            number: Decimal::from_str(&required::<String>(&row, "number", &id)?)?,
            price: amount(&row, "price", &id)?,
            fee: amount(&row, "fee", &id)?,
            provider: required(&row, "provider", &id)?,
            settlement_date: row.try_get("settlement_date")?,
            tax_paid: match row.try_get::<Option<String>, _>("tax_paid")? {
                Some(_) => Some(amount(&row, "tax_paid", &id)?),
                None => None,
            },
        }),
        "Split" => StockAction::Split(StockSplit {
            before: required(&row, "split_before", &id)?,
            after: required(&row, "split_after", &id)?,
        }),
        _ => {
            return Err(RepositoryError::InvalidStockRecord(format!(
                "unknown action '{action}' for ID '{id}'"
            )));
        }
    };

    Ok(Stock {
        id,
        time,
        ticker,
        action,
    })
}

fn amount(row: &SqliteRow, field: &str, id: &str) -> Result<Amount> {
    Ok(Amount {
        value: Decimal::from_str(&required::<String>(row, field, id)?)?,
        currency: serde_plain::from_str(&required::<String>(
            row,
            &format!("{field}_currency"),
            id,
        )?)?,
    })
}

fn required<'r, T>(row: &'r SqliteRow, field: &str, id: &str) -> Result<T>
where
    T: sqlx::Decode<'r, Sqlite> + sqlx::Type<Sqlite>,
{
    row.try_get::<Option<T>, _>(field)?.ok_or_else(|| {
        RepositoryError::InvalidStockRecord(format!("missing {field} for ID '{id}'"))
    })
}
