use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    client::{
        client::{CloudClient, CloudFile},
        error::ClientError,
    },
    local::db::FsNode,
};

use super::{file::DiscordFile, net::DiscordNetClient};

/// Virtual file host
pub struct DiscordClient {
    net_client: Arc<DiscordNetClient>,
}

impl DiscordClient {
    pub fn new() -> Result<Self, ClientError> {
        Ok(Self {
            net_client: Arc::new(DiscordNetClient::new()?),
        })
    }
}

#[async_trait]
impl CloudClient for DiscordClient {
    async fn create_file(&self, node: FsNode) -> Box<dyn CloudFile> {
        Box::new(DiscordFile::new(self.net_client.clone(), node))
    }
}
