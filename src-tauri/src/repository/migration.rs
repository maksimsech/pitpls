use tauri_plugin_sql::{Migration, MigrationKind};

pub const DB_URL: &str = "sqlite:pitpls.db";

pub fn migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "init",
        sql: r"
            CREATE TABLE IF NOT EXISTS rates(
                date DATE NOT NULL,
                currency TEXT NOT NULL,
                rate TEXT NOT NULL,
                PRIMARY KEY (date, currency)
            );
            CREATE TABLE IF NOT EXISTS cryptos(
                id TEXT PRIMARY KEY,
                date DATE NOT NULL,
                value TEXT NOT NULL,
                value_currency TEXT NOT NULL,
                fee TEXT NOT NULL,
                fee_currency TEXT NOT NULL,
                action TEXT NOT NULL,
                provider TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS dividends(
                id TEXT PRIMARY KEY,
                date DATE NOT NULL,
                ticker TEXT NOT NULL,
                value TEXT NOT NULL,
                value_currency TEXT NOT NULL,
                tax_paid TEXT NOT NULL,
                tax_paid_currency TEXT NOT NULL,
                country TEXT NOT NULL,
                provider TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS interests(
                id TEXT PRIMARY KEY,
                date DATE NOT NULL,
                value TEXT NOT NULL,
                value_currency TEXT NOT NULL,
                provider TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS years(
                year INTEGER PRIMARY KEY
            );
",
        kind: MigrationKind::Up,
    }]
}
