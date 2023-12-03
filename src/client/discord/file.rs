use std::{
    cmp::min,
    io::{Read, Write},
    sync::Arc,
};

use log::{debug, error, trace};

use crate::{
    client::error::ClientError,
    local::{
        db::{FsDatabase, FsNode},
        error::FsError,
    },
};

use super::net::DiscordNetClient;

pub const DISCORD_BLOCK_SIZE: usize = 25 * 1024 * 1024;

/// Virtual file hosted on Discord
pub struct DiscordFileWrite {
    buffer: Vec<u8>,
    total_size: i64,
    node: FsNode,
    prev_id: Option<String>,
    client: Arc<DiscordNetClient>,
    db: Arc<FsDatabase>,
}

impl DiscordFileWrite {
    pub fn new(client: Arc<DiscordNetClient>, db: Arc<FsDatabase>, node: FsNode) -> Self {
        DiscordFileWrite {
            buffer: Vec::with_capacity(DISCORD_BLOCK_SIZE),
            total_size: 0,
            node,
            prev_id: None,
            client,
            db,
        }
    }

    fn upload_buffer(&self) -> Result<String, ClientError> {
        self.client.rt.block_on(async {
            self.client
                .create_message(
                    &self.client.channel_id,
                    &self.buffer.as_slice(),
                    &self.prev_id,
                )
                .await
        })
    }
}

impl Write for DiscordFileWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.total_size += buf.len() as i64;

        // Need to upload a block
        if self.buffer.len() + buf.len() > DISCORD_BLOCK_SIZE {
            let space = DISCORD_BLOCK_SIZE - &self.buffer.len();
            let slice = &buf[..space];
            slice.iter().for_each(|b| self.buffer.push(*b));

            // Upload
            let message_id = self.upload_buffer()?;

            self.prev_id = Some(message_id);
            self.buffer.clear();
            let slice = &buf[space..];
            slice.iter().for_each(|b| self.buffer.push(*b));
        } else {
            buf.iter().for_each(|b| self.buffer.push(*b));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.buffer.len() > 0 {
            let message_id = self.upload_buffer()?;
            self.client.rt.block_on(async {
                self.db
                    .set_node_cloud_id(&self.node.id, &message_id, self.total_size)
                    .await
            })?;
        }
        Ok(())
    }
}

pub struct DiscordFileRead {
    buffer: Vec<u8>,
    node: FsNode,
    client: Arc<DiscordNetClient>,
    file_ids: Vec<u64>,
    current_index: usize,
}

impl DiscordFileRead {
    pub fn new(client: Arc<DiscordNetClient>, node: FsNode) -> Result<Self, FsError> {
        let ids: Result<Vec<u64>, FsError> = client.rt.block_on(async {
            let cloud_id = node.cloud_id.as_ref().ok_or_else(|| {
                FsError::DatabaseError(crate::local::error::DbError::Other(
                    "Cloud id not set".to_string(),
                ))
            })?;
            client
                .get_file_chain(&client.channel_id, &cloud_id)
                .await
                .map_err(|e| FsError::ClientError(e))
        });
        debug!("file ids: {:?}", ids);
        Ok(Self {
            node,
            client,
            file_ids: ids?,
            buffer: Vec::with_capacity(DISCORD_BLOCK_SIZE),
            current_index: 0,
        })
    }
}

impl std::io::Read for DiscordFileRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_size = buf.len();
        let mut copied: usize = 0;
        trace!("read buffer size: {:?}", read_size);

        // Clear buffer first
        if !self.buffer.is_empty() {
            let buf_size = self.buffer.len();
            let copy_size = min(buf_size, read_size);
            trace!("clearing read buffer: {:?}/{:?} bytes", copy_size, buf_size);
            buf[..copy_size].clone_from_slice(&self.buffer[..copy_size]);

            // Left over buffer
            if buf_size >= copy_size {
                let leftover = self.buffer[copy_size..].to_vec();
                self.buffer.clear();
                self.buffer.clone_from(&leftover);
                return Ok(copy_size);
            }

            copied += copy_size
        }

        // Or need to keep reading
        while read_size - copied > 0 && self.current_index < self.file_ids.len() {
            // Fill buffer with next chunk
            let next_id: String = self
                .file_ids
                .get(self.current_index)
                .unwrap_or(&0)
                .to_string();
            debug!("downloading id: {:?}", next_id);
            self.client.rt.block_on(async {
                self.client
                    .download_file(&self.client.channel_id, &next_id, &mut self.buffer)
                    .await
            })?;

            // Copy to output buffer
            let buf_size = self.buffer.len();
            let copy_size = min(buf_size, read_size - copied);
            buf[copied..copied + copy_size].clone_from_slice(&self.buffer[..copy_size]);

            // Left over buffer
            if buf_size > copy_size {
                let leftover = self.buffer[copy_size..].to_vec();
                self.buffer.clear();
                self.buffer.clone_from(&leftover);
            }

            copied += copy_size;
            self.current_index += 1;
        }

        Ok(copied)
    }
}
