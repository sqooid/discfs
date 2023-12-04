use std::io::{Read, Write};

use async_trait::async_trait;

use crate::local::{db::FsNode, error::FsError};

use super::error::ClientError;

#[async_trait]
pub trait CloudClient {
    fn open_file_write(&self, node: FsNode) -> Box<dyn Write>;
    fn open_file_read(&self, node: FsNode) -> Result<Box<dyn Read>, FsError>;
}
