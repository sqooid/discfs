use std::env;

use log::{debug, error};
use reqwest::{header, multipart, ClientBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Handle;

use crate::client::error::ClientError;

#[derive(Debug, Deserialize)]
struct DiscordMessage {
    id: String,
}

pub struct DiscordNetClient {
    client: reqwest::Client,
    url: String,
    pub channel_id: String,
    pub rt: Handle,
}

impl DiscordNetClient {
    pub fn new(rt: Handle) -> Result<Self, ClientError> {
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
            channel_id: env::var("CHANNEL_ID")
                .map_err(|e| ClientError::Initialization(e.to_string()))?,
            rt,
        });
    }

    /// Send a message to specified channel and if part of a larger file, link the previous chunk as a reply.
    /// Returns the id of the created message for future reference
    pub async fn create_message(
        &self,
        channel_id: &str,
        file: &[u8],
        reply_id: &Option<String>,
    ) -> Result<String, ClientError> {
        let mut form_data = multipart::Form::new();

        let part = multipart::Part::bytes(file.to_owned()).file_name("file.txt");
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

#[cfg(test)]
mod test {
    use crate::client::discord::file::DISCORD_BLOCK_SIZE;

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_create_message() {
        init();
        let client = DiscordNetClient::new(Handle::current()).unwrap();
        let result = client
            .create_message(
                &env::var("CHANNEL_ID").unwrap(),
                &vec![0; DISCORD_BLOCK_SIZE],
                &None,
            )
            .await;
        result.unwrap();
    }
}
