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

impl From<ClientError> for std::io::Error {
    fn from(value: ClientError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, value)
    }
}
