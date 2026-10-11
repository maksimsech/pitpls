use pitpls_core::{common::Mic, stock::Instrument};
use sqlx::{Row, SqlitePool};

use super::{RepositoryError, Result};

pub struct InstrumentRepository {
    db: SqlitePool,
}

impl InstrumentRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn list(&self) -> Result<Vec<Instrument>> {
        sqlx::query(
            "SELECT ticker, provider, exchange, isin FROM stock_instruments ORDER BY ticker, provider",
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|row| {
            let ticker: String = row.try_get("ticker")?;
            let provider: String = row.try_get("provider")?;
            let exchange = row
                .try_get::<Option<String>, _>("exchange")?
                .map(|exchange| {
                    exchange.parse::<Mic>().map_err(|_| {
                        RepositoryError::InvalidStockRecord(format!(
                            "invalid exchange '{exchange}' for {ticker} ({provider})"
                        ))
                    })
                })
                .transpose()?;
            let isin: Option<String> = row.try_get("isin")?;

            Ok(Instrument {
                ticker,
                provider,
                exchange,
                isin,
            })
        })
        .collect()
    }

    pub async fn upsert_from_import(&self, ticker: &str, provider: &str, isin: &str) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO stock_instruments(ticker, provider, isin)
            VALUES (?, ?, ?)
            ON CONFLICT(ticker, provider) DO UPDATE SET isin = excluded.isin
            WHERE stock_instruments.isin IS NULL
            ",
        )
        .bind(ticker)
        .bind(provider)
        .bind(isin)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn set_exchange(
        &self,
        ticker: &str,
        provider: &str,
        exchange: Option<Mic>,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO stock_instruments(ticker, provider, exchange)
            VALUES (?, ?, ?)
            ON CONFLICT(ticker, provider) DO UPDATE SET exchange = excluded.exchange
            ",
        )
        .bind(ticker)
        .bind(provider)
        .bind(exchange.map(|mic| mic.to_string()))
        .execute(&self.db)
        .await?;
        Ok(())
    }
}
