use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("DB connection error: {0}")]
    ConnectionError(String),

    #[error("Query error")]
    QueryError(#[from] sqlx::error::Error),
}
