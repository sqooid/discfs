use std::env;

use sqlx::{Database, Pool, Sqlite, SqlitePool};

use super::error::DbError;

pub struct FsDatabase<DB>
where
    DB: Database,
{
    connection: Pool<DB>,
}

impl FsDatabase<Sqlite> {
    pub async fn new() -> Result<Self, DbError> {
        let db_url =
            env::var("DATABASE_URL").map_err(|e| DbError::ConnectionError(e.to_string()))?;

        return Ok(Self {
            connection: SqlitePool::connect(&db_url)
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
