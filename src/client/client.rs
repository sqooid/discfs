use std::io::{Read, Write};

use async_trait::async_trait;

use crate::local::db::FsNode;

use super::error::ClientError;

pub trait CloudFile: Read + Write {
    fn node(&self) -> &FsNode;
}

#[async_trait]
pub trait CloudClient {
    async fn create_file(&self, node: FsNode) -> Box<dyn CloudFile>;
}
