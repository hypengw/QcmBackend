use prost;

use sea_orm::error as orm_error;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("{0}")]
    Internal(#[from] anyhow::Error),
    #[error("Decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Unsupported message type: {0}")]
    UnsupportedMessageType(i32),
    #[error("Unknown message type: {0}")]
    UnknownMessageType(i32),
    #[error("Unexpected payload for message type: {0}")]
    UnexpectedPayload(i32),
    #[error("Missing fields: {0}")]
    MissingFields(String),
    #[error("No such provider type: {0}")]
    NoSuchProviderType(String),
    #[error("No such provider type: {0}")]
    Db(#[from] orm_error::DbErr),
    #[error("")]
    None,
}

impl<T> From<SendError<T>> for ProcessError
where
    T: Send + Sync + 'static,
{
    fn from(e: SendError<T>) -> Self {
        ProcessError::Internal(e.into())
    }
}
