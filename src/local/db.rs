use std::{env, str::FromStr};

use log::info;
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

        let connection = SqlitePool::connect_with(connection_options)
            .await
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;

        {
            let initialized =
                sqlx::query!("select name from sqlite_master where type='table' and name='files'")
                    .fetch_all(&connection)
                    .await;
            match initialized {
                Ok(r) => {
                    if r.len() == 0 {
                        Self::initialise_db(&connection).await?;
                    }
                }
                Err(_) => Self::initialise_db(&connection).await?,
            };
        }

        return Ok(Self { connection });
    }

    async fn initialise_db(connection: &Pool<Sqlite>) -> Result<(), DbError> {
        info!("initializing database for the first time");
        let _ = sqlx::query(
            "
create table files (
    id integer primary key,
    name text,
    ctime integer,
    atime integer,
    directory_id integer not null,
    foreign key(directory_id) references directories(id)
); 

create table directories (
    id integer primary key,
    name text,
    parent_id integer
);
            ",
        )
        .execute(connection)
        .await
        .map_err(|e| DbError::QueryError(e))?;
        let _ = sqlx::query!(
            "
insert into directories (name, parent_id) values (null, null);
            ",
        )
        .execute(connection)
        .await
        .map_err(|e| DbError::QueryError(e))?;

        Ok(())
    }
}

pub struct DbFile {}

#[cfg(test)]
mod tests {
    use super::FsDatabase;

    #[tokio::test]
    async fn test_connection() {
        let db = FsDatabase::new().await.unwrap();
        let tables = sqlx::query!("create table test (id int)");
    }
}
