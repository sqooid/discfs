use std::{ffi::OsStr, str::FromStr, time::SystemTime};

use log::info;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite, SqlitePool};

use crate::util::time::time_to_float;

use super::error::DbError;

pub struct FsDatabase {
    pub connection: Pool<Sqlite>,
}

impl FsDatabase {
    pub async fn new(path: &str) -> Result<Self, DbError> {
        let db_url = format!("sqlite:{}", path);
        let connection_options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);

        let connection = SqlitePool::connect_with(connection_options).await?;

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
        .await?;

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
        .await?;
        Ok(node)
    }
    pub async fn get_node_by_id(&self, id: u64) -> Result<Option<FsNode>, DbError> {
        let id = id as i64;
        let node = sqlx::query_as!(FsNode, "select * from node where id=?", id)
            .fetch_optional(&self.connection)
            .await?;
        Ok(node)
    }

    pub async fn create_node(
        &self,
        parent: u64,
        name: &OsStr,
        directory: bool,
    ) -> Result<FsNode, DbError> {
        let parent_id = parent as i64;
        let name = name.to_string_lossy();
        let node = sqlx::query_as!(
            FsNode,
            "select * from node where parent=? and name=?",
            parent_id,
            name
        )
        .fetch_optional(&self.connection)
        .await?;
        if let Some(node) = node {
            return Err(DbError::Exists(node.id, name.to_string()));
        }
        let ctime = time_to_float(&SystemTime::now()).map_err(|e| DbError::Other(e.to_string()))?;
        let new_node = sqlx::query_as!(
            FsNode,
            "insert into node (parent, name, directory, ctime) values (?, ?, ?, ?); select * from node where parent=? and name=?",
            parent_id,
            name,
            directory,
            ctime,
            parent_id,
            name,
        )
        .fetch_one(&self.connection)
        .await?;

        Ok(new_node)
    }

    pub async fn set_node_cloud_id(
        &self,
        id: &i64,
        cloud_id: &str,
        size: i64,
    ) -> Result<(), DbError> {
        let result = sqlx::query!(
            "update node set cloud_id=?, size=? where id=?",
            cloud_id,
            size,
            id,
        )
        .execute(&self.connection)
        .await?;
        if result.rows_affected() == 0 {
            Err(DbError::DoesNotExist(*id))
        } else {
            Ok(())
        }
    }

    pub async fn get_nodes_by_parent(&self, parent_id: i64) -> Result<Vec<FsNode>, DbError> {
        let result = sqlx::query_as!(FsNode, "select * from node where parent=?", parent_id)
            .fetch_all(&self.connection)
            .await?;
        Ok(result)
    }

    pub async fn delete_dir(&self, parent_id: i64, name: &str) -> Result<u64, DbError> {
        let result = sqlx::query!(
            "delete from node where parent=? and name=? and directory=1",
            parent_id,
            name
        )
        .execute(&self.connection)
        .await?;
        Ok(result.rows_affected())
    }
}

#[derive(Debug, Clone)]
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
