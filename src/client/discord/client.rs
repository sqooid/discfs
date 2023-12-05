use std::{env, sync::Arc};

use async_trait::async_trait;
use tokio::runtime::Handle;

use crate::{
    client::{
        client::{CloudClient, CloudRead, CloudWrite},
        error::ClientError,
    },
    encryption::aes::Aes,
    local::{
        db::{FsDatabase, FsNode},
        error::FsError,
    },
};

use super::{
    file::{DiscordFileRead, DiscordFileWrite},
    net::DiscordNetClient,
};

/// Virtual file host
pub struct DiscordClientInner {
    pub net: DiscordNetClient,
    pub db: Arc<FsDatabase>,
    pub aes: Aes,
}

pub struct DiscordClient {
    inner: Arc<DiscordClientInner>,
}

impl DiscordClient {
    pub fn new(rt: Handle, db: Arc<FsDatabase>) -> Result<Self, ClientError> {
        let aes = Aes::from_env("SECRET_KEY")?;
        Ok(Self {
            inner: Arc::new(DiscordClientInner {
                net: DiscordNetClient::new(rt)?,
                db,
                aes,
            }),
        })
    }
}

#[async_trait]
impl CloudClient for DiscordClient {
    async fn open_file_write(&self, node: FsNode) -> Box<dyn CloudWrite> {
        Box::new(DiscordFileWrite::new(self.inner.clone(), node))
    }

    async fn open_file_read(&self, node: FsNode) -> Result<Box<dyn CloudRead>, FsError> {
        Ok(Box::new(
            DiscordFileRead::new(self.inner.clone(), node).await?,
        ))
    }
}
