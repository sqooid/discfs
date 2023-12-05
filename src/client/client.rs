use async_trait::async_trait;

use crate::{
    local::{db::FsNode, error::FsError},
    util::async_file::{AsyncRead, AsyncWrite},
};

#[async_trait]
pub trait CloudClient: Send + Sync {
    async fn open_file_write(&self, node: FsNode) -> Box<dyn CloudWrite>;
    async fn open_file_read(&self, node: FsNode) -> Result<Box<dyn CloudRead>, FsError>;
}

pub trait CloudWrite: AsyncWrite + Send + Sync {
    /// Essentially a hook at the end of a write operation.
    /// Useful for logging
    fn finish(&self) -> ();
}

pub trait CloudRead: AsyncRead + Send + Sync {
    /// Essentially a hook at the end of a read operation.
    /// Useful for logging
    fn finish(&self) -> ();
}
