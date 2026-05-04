use thiserror::Error;

pub mod crypto;
pub mod dividend;
pub mod interest;
pub mod rate;
pub mod settings;
pub mod year;

pub type Result<T> = std::result::Result<T, RepositoryError>;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_plain::Error),

    #[error("Decimal parse error: {0}")]
    Decimal(#[from] rust_decimal::Error),
}

impl RepositoryError {
    pub fn is_unique_violation(&self) -> bool {
        matches!(
            self,
            Self::Database(sqlx::Error::Database(error)) if error.is_unique_violation()
        )
    }
}
