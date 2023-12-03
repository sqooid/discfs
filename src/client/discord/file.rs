use std::{
    io::{Read, Write},
    sync::Arc,
};

use crate::{client::client::CloudFile, local::db::FsNode};

use super::net::DiscordNetClient;

pub const DISCORD_BLOCK_SIZE: usize = 25 * 1024 * 1024;
/// Virtual file hosted on Discord
pub struct DiscordFile {
    buffer: Vec<u8>,
    buf_size: usize,
    node: FsNode,
    prev_id: Option<String>,
    client: Arc<DiscordNetClient>,
}

impl DiscordFile {
    pub fn new(client: Arc<DiscordNetClient>, node: FsNode) -> Self {
        DiscordFile {
            buffer: vec![0; DISCORD_BLOCK_SIZE],
            buf_size: 0,
            node,
            prev_id: None,
            client,
        }
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
        let buf_size = buf.len();
        let total_buf_size = self.buf_size + buf_size;
        // Need to upload a block
        if total_buf_size > DISCORD_BLOCK_SIZE {
            let slice = &buf[..DISCORD_BLOCK_SIZE - &self.buffer.len()];
            slice.iter().for_each(|b| self.buffer.push(*b));
        } else {
            buf.iter().for_each(|b| self.buffer.push(*b));
        }
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}
