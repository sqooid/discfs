use reqwest::{header, ClientBuilder};

use super::{
    client::{CloudClient, CloudFile},
    error::ClientError,
};
use std::{
    env,
    io::{Read, Write},
};

/// Virtual file hosted on Discord
pub struct DiscordFile {
    buffer: [u8; 25 * 1024 * 1024],
}

impl CloudFile for DiscordFile {}

impl Read for DiscordFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}
impl Write for DiscordFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

/// Virtual file host
pub struct DiscordClient {
    client: reqwest::Client,
}

impl DiscordClient {
    pub fn new() -> Result<Self, ClientError> {
        // Set up discord bot token
        let mut default_headers = header::HeaderMap::new();
        let discord_token = env::var("DISCORD_TOKEN")
            .map_err(|e| ClientError::ClientInitializationFailed(e.to_string()))?;
        let auth_header = format!("Bot {}", discord_token);
        default_headers.insert(
            "Authorization",
            header::HeaderValue::from_str(auth_header.as_str())
                .map_err(|e| ClientError::ClientInitializationFailed(e.to_string()))?,
        );

        // Set client default headers
        let discord_client = ClientBuilder::new()
            .user_agent("DiscordBot (custom, 1)")
            .default_headers(default_headers)
            .build()
            .map_err(|e| ClientError::ClientInitializationFailed(e.to_string()))?;

        return Ok(Self {
            client: discord_client,
        });
    }
}

impl CloudClient for DiscordClient {
    fn list_files(path: &str) {
        todo!()
    }
}
