use serde::Serialize;
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize, Type)]
pub struct Warnings {
    rates_empty: bool,
    has_records_in_year: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn get_warnings(
    state: State<'_, AppState>,
    year: Option<i32>,
) -> Result<Warnings, String> {
    let rates = state
        .rate_repo()
        .load_all()
        .await
        .map_err(|e| e.to_string())?;

    let has_records = any_records_in_year(&state, year)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Warnings {
        rates_empty: rates.is_empty(),
        has_records_in_year: has_records,
    })
}

async fn any_records_in_year(
    state: &AppState,
    year: Option<i32>,
) -> Result<bool, sqlx::Error> {
    let pool = state.db_pool();
    let tables = ["cryptos", "dividends", "interests"];
    for table in tables {
        if table_has_records(pool, table, year).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn table_has_records(
    pool: &SqlitePool,
    table: &str,
    year: Option<i32>,
) -> Result<bool, sqlx::Error> {
    let exists: i64 = match year {
        None => {
            sqlx::query_scalar(&format!(
                "SELECT EXISTS(SELECT 1 FROM {table}) AS e"
            ))
            .fetch_one(pool)
            .await?
        }
        Some(y) => {
            sqlx::query_scalar(&format!(
                "SELECT EXISTS(SELECT 1 FROM {table} WHERE date BETWEEN ? AND ?) AS e"
            ))
            .bind(format!("{y:04}-01-01"))
            .bind(format!("{y:04}-12-31"))
            .fetch_one(pool)
            .await?
        }
    };
    Ok(exists != 0)
}
