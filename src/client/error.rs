use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Client initialization failed: {0}")]
    Initialization(String),

    #[error("Request client error: {0}")]
    RequestClient(#[from] reqwest::Error),

    #[error("Request error: {0}")]
    RequestValue(String),
}
