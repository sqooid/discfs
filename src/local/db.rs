use std::{env, str::FromStr};

use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Pool, Sqlite, SqlitePool};

use super::error::DbError;

pub struct FsDatabase {
    connection: Pool<Sqlite>,
}

impl FsDatabase {
    pub async fn new() -> Result<Self, DbError> {
        let db_url = env::var("SQLITE_URL").map_err(|e| DbError::ConnectionError(e.to_string()))?;
        let connection_options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| DbError::ConnectionError(e.to_string()))?
            .create_if_missing(true);

        return Ok(Self {
            connection: SqlitePool::connect_with(connection_options)
                .await
                .map_err(|e| DbError::ConnectionError(e.to_string()))?,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::FsDatabase;

    #[tokio::test]
    async fn test_connection() {
        let db = FsDatabase::new().await.unwrap();
        let tables = sqlx::query!("create table test (id int)");
    }
}
