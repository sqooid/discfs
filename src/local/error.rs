use thiserror::Error;

use crate::client::error::ClientError;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("DB connection error: {0}")]
    ConnectionError(String),

    #[error("Query error {0}")]
    SqlxError(#[from] sqlx::error::Error),

    #[error("Node already exists: {1} ({0})")]
    Exists(i64, String),

    #[error("Node does not exist {0}")]
    DoesNotExist(i64),

    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum FsError {
    #[error("Error with runtime: {0}")]
    RuntimeError(String),

    #[error("Error with database: {0}")]
    DatabaseError(#[from] DbError),

    #[error("Erro with system time: {0}")]
    TimeError(String),

    #[error("Client error: {0}")]
    ClientError(#[from] ClientError),
}

impl From<DbError> for std::io::Error {
    fn from(value: DbError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, value)
    }
}
