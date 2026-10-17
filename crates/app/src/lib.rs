use pitpls_db::Database;
use reqwest::Client;

#[cfg(test)]
mod tests;
pub mod use_case;

pub struct App {
    pub db: Database,
    pub api_client: Client,
}

impl App {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            api_client: Client::new(),
        }
    }
}
