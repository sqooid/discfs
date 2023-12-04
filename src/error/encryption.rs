use thiserror::Error;

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Unknown error")]
    AesUnknown,
}

impl From<ring::error::Unspecified> for EncryptionError {
    fn from(value: ring::error::Unspecified) -> Self {
        Self::AesUnknown
    }
}
