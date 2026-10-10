use thiserror::Error;

pub mod crypto;
pub mod dividend;
pub mod interest;
pub mod rate;
pub mod settings;
pub mod year;

pub type Result<T> = std::result::Result<T, RepositoryError>;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum RepositoryError {
    Database(#[from] sqlx::Error),

    Serialization(#[from] serde_plain::Error),

    Decimal(#[from] rust_decimal::Error),

    InvalidYear(i32),
}

impl RepositoryError {
    pub fn is_unique_violation(&self) -> bool {
        matches!(
            self,
            Self::Database(sqlx::Error::Database(error)) if error.is_unique_violation()
        )
    }
}
