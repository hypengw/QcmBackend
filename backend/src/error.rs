use prost;

use hyper;
use sea_orm::error as orm_error;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("{0}")]
    Internal(#[from] anyhow::Error),
    #[error("Encode error: {0}")]
    Encode(#[from] prost::EncodeError),
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
    #[error("Database error: {0}")]
    Db(#[from] orm_error::DbErr),
    #[error("Hyper body error: {0}")]
    HyperBody(#[from] hyper::Error),
    #[error("Wrong id: {0}")]
    WrongId(String),
    #[error("No such library: {0}")]
    NoSuchLibrary(String),
    #[error("No such provider: {0}")]
    NoSuchProvider(String),
    #[error("No such album: {0}")]
    NoSuchAlbum(String),
    #[error("No such song: {0}")]
    NoSuchSong(String),
    #[error("No such artist: {0}")]
    NoSuchArtist(String),
    #[error("No such mix: {0}")]
    NoSuchMix(String),
    #[error("No such item type: {0}")]
    NoSuchItemType(String),
    #[error("No such image type: {0}")]
    NoSuchImageType(String),
    #[error("Unsupported item type: {0}")]
    UnsupportedItemType(String),
    #[error("Infallible")]
    Infallible(#[from] std::convert::Infallible),
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

impl From<qcm_core::error::ConnectError> for ProcessError {
    fn from(e: qcm_core::error::ConnectError) -> Self {
        use qcm_core::error::ConnectError;
        match e {
            ConnectError::Infallible(e) => ProcessError::Infallible(e),
            e => ProcessError::Internal(e.into()),
        }
    }
}

#[derive(Debug, Error)]
pub enum HttpError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("Hyper body error: {0}")]
    HyperBody(#[from] hyper::Error),
    #[error("Infallible")]
    Infallible(#[from] std::convert::Infallible),
}

impl From<HttpError> for ProcessError {
    fn from(e: HttpError) -> Self {
        match e {
            HttpError::Reqwest(e) => ProcessError::Internal(e.into()),
            HttpError::HyperBody(e) => ProcessError::HyperBody(e),
            HttpError::Infallible(e) => ProcessError::Infallible(e),
        }
    }
}
