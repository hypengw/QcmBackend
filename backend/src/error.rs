use prost;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("Decode error: {0}")]
    DecodeError(#[from] prost::DecodeError),
    #[error("Unknown message type: {0}")]
    UnknownMessageType(i32),
    #[error("Unexpected payload for message type: {0}")]
    UnexpectedPayload(i32),
    #[error("Missing fields: {0}")]
    MissingFields(String),
    #[error("")]
    None,
}
