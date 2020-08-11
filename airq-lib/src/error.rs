use std::fmt;

use block_modes::BlockModeError;

#[derive(Debug)]
pub enum Error {
    RequestError(reqwest::Error),
    Base64Error(base64::DecodeError),
    DecryptionError(BlockModeError),
    JsonError(serde_json::Error)
}

pub type Result<T> = std::result::Result<T, Error>;
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::RequestError(e)
    }
}
impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Error::Base64Error(e)
    }
}
impl From<BlockModeError> for Error {
    fn from(e: BlockModeError) -> Self {
        Error::DecryptionError(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::JsonError(e.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::RequestError(e) => write!(f, "request error: {}", e),
            Error::Base64Error(e) => write!(f, "base64 decoding error: {}", e),
            Error::DecryptionError(e) => write!(f, "decryption error: {}", e),
            Error::JsonError(e) => write!(f, "JSON deserialization error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Error::RequestError(e) => e,
            Error::Base64Error(e) => e,
            Error::DecryptionError(e) => e,
            Error::JsonError(e) => e,
        })
    }
}

