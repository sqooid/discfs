use std::{
    io::{Read, Write},
    sync::Arc,
};

use async_trait::async_trait;
use tokio::runtime::Handle;

use crate::{
    client::{
        client::{CloudClient, CloudRead, CloudWrite},
        error::ClientError,
    },
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
pub struct DiscordClient {
    net_client: Arc<DiscordNetClient>,
    db: Arc<FsDatabase>,
}

impl DiscordClient {
    pub fn new(rt: Handle, db: Arc<FsDatabase>) -> Result<Self, ClientError> {
        Ok(Self {
            net_client: Arc::new(DiscordNetClient::new(rt)?),
            db,
        })
    }
}

#[async_trait]
impl CloudClient for DiscordClient {
    fn open_file_write(&self, node: FsNode) -> Box<dyn CloudWrite> {
        Box::new(DiscordFileWrite::new(
            self.net_client.clone(),
            self.db.clone(),
            node,
        ))
    }
    fn open_file_read(&self, node: FsNode) -> Result<Box<dyn CloudRead>, FsError> {
        Ok(Box::new(DiscordFileRead::new(
            self.net_client.clone(),
            node,
        )?))
    }
}
