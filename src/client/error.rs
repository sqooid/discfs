use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Client initialization failed: {0}")]
    ClientInitializationFailed(String),
}
