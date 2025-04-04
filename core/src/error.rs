use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error("Not auth")]
    NotAuth,
    #[error("Unknown base")]
    UnknownBase,
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Request error: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("Infallible")]
    Infallible(#[from] std::convert::Infallible),
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error(transparent)]
    Connect(#[from] ConnectError),
    #[error("Database error: {0}")]
    Db(#[from] sea_orm::DbErr),
}
