use std::{cmp::min, env};

use log::{debug, error, trace};
use reqwest::{header, multipart, ClientBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Handle;

use crate::{client::error::ClientError, main};

const DISCORD_FILENAME: &str = "file.bin";

#[derive(Debug, Deserialize)]
struct DiscordMessageUpload {
    id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DiscordMessageDownload {
    id: String,
    attachments: Vec<DiscordAttachment>,
    message_reference: Option<DiscordReference>,
    referenced_message: Option<Box<DiscordMessageDownload>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DiscordReference {
    message_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DiscordAttachment {
    id: String,
}

pub struct DiscordNetClient {
    client: reqwest::Client,
    url: String,
    files_url: String,
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
            url: env::var("DISCORD_URL")
                .unwrap_or_else(|_| "https://discord.com/api/v10".to_string()),
            files_url: env::var("DISCORD_FILES_URL")
                .unwrap_or_else(|_| "https://cdn.discordapp.com/attachments".to_string()),
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

        let part = multipart::Part::bytes(file.to_owned()).file_name(DISCORD_FILENAME);
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
        let body = request.json::<DiscordMessageUpload>().await?;
        debug!("uploaded message: {}", body.id);

        Ok(body.id)
    }

    pub async fn get_file_chain(
        &self,
        channel_id: &str,
        end_id: &str,
    ) -> Result<Vec<u64>, ClientError> {
        let mut reverse_ids = vec![];

        let mut send_id = Some(end_id.to_owned());
        while let Some(id) = &send_id {
            let builder = self
                .client
                .get(format!(
                    "{}/channels/{}/messages/{}",
                    self.url, channel_id, id
                ))
                .build()?;
            debug!("download request: {:?}", builder);
            let response = self.client.execute(builder).await?;

            let body: DiscordMessageDownload = response.json().await?;
            trace!(
                "download body: {}",
                serde_json::to_string_pretty(&body).unwrap()
            );

            // Can add ids 2 at a time due to message_reference being included
            if let Some(attachment) = &body.attachments.get(0) {
                reverse_ids.push(u64::from_str_radix(&attachment.id, 10)?);
            }
            if let Some(message) = body.referenced_message {
                if let Some(attachment) = message.attachments.get(0) {
                    reverse_ids.push(u64::from_str_radix(&attachment.id, 10)?);
                }

                // Set next query
                send_id = message.message_reference.map(|m| m.message_id);
            } else {
                send_id = None;
            }
        }

        Ok(reverse_ids.into_iter().rev().collect())
    }

    /// Download discord attachment.
    /// Fills provided buffer with downloaded bytes and returns valid slice
    pub async fn download_file<'a>(
        &self,
        channel_id: &str,
        attachment_id: &str,
        buffer: &'a mut Vec<u8>,
    ) -> Result<&'a [u8], ClientError> {
        let response = self
            .client
            .get(format!(
                "{}/{}/{}/{}",
                self.files_url, channel_id, attachment_id, DISCORD_FILENAME
            ))
            .send()
            .await?;
        let body = response.bytes().await?;
        buffer.clear();
        buffer.extend(&body[..body.len()]);
        Ok(&buffer[..body.len()])
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
    type TestResult = Result<(), Box<dyn Error>>;

    #[tokio::test]
    async fn test_create_message() -> TestResult {
        init();
        let client = DiscordNetClient::new(Handle::current())?;
        let _result = client
            .create_message(&env::var("CHANNEL_ID")?, &vec![0; 6], &None)
            .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_chain() -> TestResult {
        init();
        let client = DiscordNetClient::new(Handle::current())?;
        let result = client
            .get_file_chain(&env::var("CHANNEL_ID")?, "1180822826584912006")
            .await;
        debug!("chain: {:?}", result?);
        Ok(())
    }

    #[tokio::test]
    async fn test_download() -> TestResult {
        init();
        let client = DiscordNetClient::new(Handle::current())?;
        let mut buffer: Vec<u8> = vec![];
        let _result = client
            .download_file(&env::var("CHANNEL_ID")?, "1180822826329055292", &mut buffer)
            .await;
        debug!("downloaded: {:?}", buffer.len());
        Ok(())
    }
}
