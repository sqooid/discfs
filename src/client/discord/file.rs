use std::{
    io::{Read, Write},
    sync::Arc,
};

use log::error;

use crate::{
    client::{client::CloudFile, error::ClientError},
    local::db::{FsDatabase, FsNode},
};

use super::net::DiscordNetClient;

pub const DISCORD_BLOCK_SIZE: usize = 25 * 1024 * 1024;
/// Virtual file hosted on Discord
pub struct DiscordFile {
    buffer: Vec<u8>,
    total_size: i64,
    node: FsNode,
    prev_id: Option<String>,
    client: Arc<DiscordNetClient>,
    db: Arc<FsDatabase>,
}

impl DiscordFile {
    pub fn new(client: Arc<DiscordNetClient>, db: Arc<FsDatabase>, node: FsNode) -> Self {
        DiscordFile {
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

impl CloudFile for DiscordFile {
    fn node(&self) -> &crate::local::db::FsNode {
        &self.node
    }
}

impl Read for DiscordFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let _buf_size = buf.len();
        todo!()
    }
}

impl Write for DiscordFile {
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
