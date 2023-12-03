use async_trait::async_trait;
use log::{debug, error};
use reqwest::{
    header::{self},
    multipart::{self},
    ClientBuilder, StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use crate::local::db::FsNode;

use super::{
    client::{CloudClient, CloudFile},
    error::ClientError,
};
use std::{
    env,
    io::{Read, Write},
    sync::Arc,
};

const DISCORD_BLOCK_SIZE: usize = 25 * 1024 * 1024;
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

pub struct DiscordNetClient {
    client: reqwest::Client,
    url: String,
}

impl DiscordNetClient {
    pub fn new() -> Result<Self, ClientError> {
        // Set up discord bot token
        let mut default_headers = header::HeaderMap::new();
        let discord_token =
            env::var("DISCORD_TOKEN").map_err(|e| ClientError::Initialization(e.to_string()))?;
        let auth_header = format!("Bot {}", discord_token);
        default_headers.insert(
            "Authorization",
            header::HeaderValue::from_str(auth_header.as_str())
                .map_err(|e| ClientError::Initialization(e.to_string()))?,
        );

        // Set client default headers
        let discord_client = ClientBuilder::new()
            .user_agent("DiscordBot (custom, 1)")
            .default_headers(default_headers)
            .build()
            .map_err(|e| ClientError::Initialization(e.to_string()))?;

        return Ok(Self {
            url: env::var("DISCORD_URL").map_err(|e| ClientError::Initialization(e.to_string()))?,
            client: discord_client,
        });
    }

    async fn create_message(
        &self,
        channel_id: &str,
        file: &[u8],
        reply_id: Option<&str>,
    ) -> Result<String, ClientError> {
        let mut form_data = multipart::Form::new();

        let part = multipart::Part::bytes(file.to_owned()).file_name("file");
        form_data = form_data.part("files[0]", part);

        if let Some(id) = reply_id {
            form_data = form_data.text(
                "payload_json",
                json!({ "message_reference": {"message_id": id} }).to_string(),
            );
        }

        let builder = self
            .client
            .post(format!("{}/channels/{}/messages", &self.url, channel_id))
            .multipart(form_data)
            .build()?;

        debug!("create message request: {:?}", &builder);

        let request = self.client.execute(builder).await?;

        debug!("create message response headers: {:?}", &request);
        let status = request.status();
        if status != StatusCode::OK {
            let body = request.json::<serde_json::Value>().await?;
            error!(
                "create message error: {}",
                serde_json::to_string_pretty(&body).unwrap_or(body.to_string())
            );
            return Err(ClientError::RequestValue(format!(
                "status: {:?}\nbody: {:?}",
                status, body
            )));
        }
        let body = request.json::<DiscordMessage>().await?;
        debug!("uploaded message: {}", body.id);

        Ok(body.id)
    }
}

#[async_trait]
impl CloudClient for DiscordClient {
    async fn create_file(&self, node: FsNode) -> Box<dyn CloudFile> {
        Box::new(DiscordFile::new(self.net_client.clone(), node))
    }
}

#[derive(Debug, Deserialize)]
struct DiscordMessage {
    id: String,
}

#[cfg(test)]
mod test {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_create_message() {
        init();
        let client = DiscordNetClient::new().unwrap();
        let result = client
            .create_message(
                &env::var("CHANNEL_ID").unwrap(),
                &vec![0; DISCORD_BLOCK_SIZE],
                None,
            )
            .await;
        result.unwrap();
    }
}
