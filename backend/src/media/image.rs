use futures_util::{Stream, StreamExt};
use hyper::{
    body::{Bytes, Incoming},
    Client, Request,
};
use qcm_core::{
    db::values::StringVec,
    model::cache::{self, ActiveModel as CacheModel},
    Result,
};
use sea_orm::*;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

pub async fn fetch_image_by_key(
    library_id: &str,
    key: &str,
    db: &DatabaseConnection,
) -> Result<impl Stream<Item = Result<Bytes, std::io::Error>>> {
    Ok(stream)
}