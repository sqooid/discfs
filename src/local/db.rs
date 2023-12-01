use std::{env, ffi::OsStr, str::FromStr};

use log::info;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite, SqlitePool};

use super::error::DbError;

pub struct FsDatabase {
    pub connection: Pool<Sqlite>,
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
                sqlx::query!("select name from sqlite_master where type='table' and name='node'")
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
create table node (
    id integer primary key,
    name text,
    size integer,
    ctime float,
    atime float,
    parent integer,
    directory boolean,
    cloud_id text,
    foreign key(parent) references node(id)
); 

insert into node (id, name, parent) values (1, null, null);
            ",
        )
        .execute(connection)
        .await
        .map_err(|e| DbError::QueryError(e))?;

        Ok(())
    }

    pub async fn get_node(&self, parent: u64, name: &OsStr) -> Result<Option<FsNode>, DbError> {
        let parent_id = parent as i64;
        let name = name.to_string_lossy();
        let node = sqlx::query_as!(
            FsNode,
            "select * from node where parent=? and name=?",
            parent_id,
            name
        )
        .fetch_optional(&self.connection)
        .await
        .map_err(|e| (DbError::QueryError(e)))?;
        Ok(node)
    }
}

pub struct FsNode {
    pub id: i64,
    pub name: Option<String>,
    pub size: Option<i64>,
    pub ctime: Option<f64>,
    pub atime: Option<f64>,
    pub parent: Option<i64>,
    pub directory: bool,
    pub cloud_id: Option<String>,
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
