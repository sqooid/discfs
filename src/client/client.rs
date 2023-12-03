use std::io::{Read, Write};

use async_trait::async_trait;

use crate::local::db::FsNode;

use super::error::ClientError;

#[async_trait]
pub trait CloudClient {
    fn create_file(&self, node: FsNode) -> Box<dyn Write>;
}
