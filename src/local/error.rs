use std::time::SystemTimeError;

use sqlx::error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("DB connection error: {0}")]
    ConnectionError(String),

    #[error("Query error")]
    QueryError(#[from] sqlx::error::Error),
}

#[derive(Error, Debug)]
pub enum FsError {
    #[error("Error with runtime: {0}")]
    RuntimeError(String),

    #[error("Error with database: {0}")]
    DatabaseError(#[from] DbError),

    #[error("Erro with system time: {0}")]
    TimeError(String),
}
