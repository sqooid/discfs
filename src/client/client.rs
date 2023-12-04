use std::io::{Read, Write};

use async_trait::async_trait;

use crate::local::{db::FsNode, error::FsError};

#[async_trait]
pub trait CloudClient {
    fn open_file_write(&self, node: FsNode) -> Box<dyn CloudWrite>;
    fn open_file_read(&self, node: FsNode) -> Result<Box<dyn CloudRead>, FsError>;
}

pub trait CloudWrite: Write {
    /// Essentially a hook at the end of a write operation.
    /// Useful for logging
    fn finish(&self) -> ();
}

pub trait CloudRead: Read {
    /// Essentially a hook at the end of a read operation.
    /// Useful for logging
    fn finish(&self) -> ();
}
