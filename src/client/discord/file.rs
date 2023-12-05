use std::{cmp::min, sync::Arc, time::SystemTime};

use async_trait::async_trait;
use log::{debug, info, trace};
use ring::aead::{MAX_TAG_LEN, NONCE_LEN};

use crate::{
    client::{
        client::{CloudRead, CloudWrite},
        error::ClientError,
    },
    local::{db::FsNode, error::FsError},
    util::async_file::{AsyncRead, AsyncWrite},
};

use super::client::DiscordClientInner;

pub const DISCORD_CONTENT_SIZE: usize = 25 * 1024 * 1024;
pub const DISCORD_BLOCK_SIZE: usize = DISCORD_CONTENT_SIZE - MAX_TAG_LEN - NONCE_LEN;

/// Virtual file hosted on Discord
pub struct DiscordFileWrite {
    buffer: Vec<u8>,
    total_size: i64,
    node: FsNode,
    prev_id: Option<String>,
    open_time: SystemTime,
    client: Arc<DiscordClientInner>,
}

impl DiscordFileWrite {
    pub fn new(client: Arc<DiscordClientInner>, node: FsNode) -> Self {
        DiscordFileWrite {
            buffer: Vec::with_capacity(DISCORD_BLOCK_SIZE),
            total_size: 0,
            node,
            prev_id: None,
            client,
            open_time: SystemTime::now(),
        }
    }

    /// Uploads a buffer with encryption.
    /// Internal buffer gets mutated in place so must be cleared to be reused
    async fn upload_buffer(&mut self) -> Result<String, ClientError> {
        // Encrypt buffer
        let id = self
            .client
            .net
            .create_message(
                &self.client.net.channel_id,
                &self.buffer.as_slice(),
                &self.prev_id,
            )
            .await?;
        self.buffer.clear();
        Ok(id)
    }
}

impl CloudWrite for DiscordFileWrite {
    fn finish(&self) -> () {
        let time = self.open_time.elapsed().unwrap_or_default().as_secs_f64();
        info!(
            "wrote {} bytes in {}s ({} MiB/s)",
            self.total_size,
            time,
            self.total_size as f64 / (1024.0 * 1024.0 * time)
        );
    }
}

#[async_trait]
impl AsyncWrite for DiscordFileWrite {
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.total_size += buf.len() as i64;

        // Need to upload a block
        if self.buffer.len() + buf.len() > DISCORD_CONTENT_SIZE {
            let space = DISCORD_CONTENT_SIZE - &self.buffer.len();
            let slice = &buf[..space];
            slice.iter().for_each(|b| self.buffer.push(*b));

            // Upload
            let message_id = self.upload_buffer().await?;

            self.prev_id = Some(message_id);
            self.buffer.clear();
            let slice = &buf[space..];
            slice.iter().for_each(|b| self.buffer.push(*b));
        } else {
            buf.iter().for_each(|b| self.buffer.push(*b));
        }
        Ok(buf.len())
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        if self.buffer.len() > 0 {
            let message_id = self.upload_buffer().await?;
            self.client
                .db
                .set_node_cloud_id(&self.node.id, &message_id, self.total_size)
                .await?;
        }
        Ok(())
    }
}

pub struct DiscordFileRead {
    buffer: Vec<u8>,
    client: Arc<DiscordClientInner>,
    file_ids: Vec<u64>,
    current_index: usize,
    open_time: SystemTime,
    total_size: u64,
}

impl DiscordFileRead {
    pub async fn new(client: Arc<DiscordClientInner>, node: FsNode) -> Result<Self, FsError> {
        let cloud_id = node.cloud_id.as_ref().ok_or_else(|| {
            FsError::DatabaseError(crate::local::error::DbError::Other(
                "Cloud id not set".to_string(),
            ))
        })?;
        let ids: Vec<u64> = client
            .net
            .get_file_chain(&client.net.channel_id, &cloud_id)
            .await
            .map_err(|e| FsError::ClientError(e))?;
        debug!("file ids: {:?}", ids);
        Ok(Self {
            client,
            file_ids: ids,
            buffer: Vec::with_capacity(DISCORD_CONTENT_SIZE),
            current_index: 0,
            open_time: SystemTime::now(),
            total_size: 0,
        })
    }
}

impl CloudRead for DiscordFileRead {
    fn finish(&self) -> () {
        let time = self.open_time.elapsed().unwrap_or_default().as_secs_f64();
        info!(
            "read {} bytes in {}s ({} MiB/s)",
            self.total_size,
            time,
            self.total_size as f64 / (1024.0 * 1024.0 * time)
        );
    }
}

#[async_trait]
impl AsyncRead for DiscordFileRead {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
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
                self.total_size += copy_size as u64;
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
            self.client
                .net
                .download_file(&self.client.net.channel_id, &next_id, &mut self.buffer)
                .await?;

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

        self.total_size += copied as u64;
        Ok(copied)
    }
}
