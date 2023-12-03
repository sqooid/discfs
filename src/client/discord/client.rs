use std::{io::Write, sync::Arc};

use async_trait::async_trait;
use tokio::runtime::Handle;

use crate::{
    client::{client::CloudClient, error::ClientError},
    local::db::{FsDatabase, FsNode},
};

use super::{file::DiscordFileWrite, net::DiscordNetClient};

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
    fn create_file(&self, node: FsNode) -> Box<dyn Write> {
        Box::new(DiscordFileWrite::new(
            self.net_client.clone(),
            self.db.clone(),
            node,
        ))
    }
}
