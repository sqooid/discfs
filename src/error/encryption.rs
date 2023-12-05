use std::env::VarError;

use base64::DecodeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Unknown error")]
    AesUnknown,

    #[error("Valid key not provided in env variable {0}")]
    InvalidKey(String),
}

impl From<ring::error::Unspecified> for EncryptionError {
    fn from(_value: ring::error::Unspecified) -> Self {
        Self::AesUnknown
    }
}

impl From<VarError> for EncryptionError {
    fn from(value: VarError) -> Self {
        Self::InvalidKey(value.to_string())
    }
}

impl From<DecodeError> for EncryptionError {
    fn from(value: DecodeError) -> Self {
        Self::InvalidKey(value.to_string())
    }
}
