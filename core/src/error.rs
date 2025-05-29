use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    ParseSubtitle(String),
    #[error("Lua error: {0}")]
    Lua(String),
    #[error("Not auth")]
    NotAuth,
    #[error("Unknown base")]
    UnknownBase,
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("UUid: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("Not Found")]
    NotFound,
    #[error("Not Implemented")]
    NotImplemented,
    #[error("Database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("Infallible")]
    Infallible(#[from] std::convert::Infallible),
}
