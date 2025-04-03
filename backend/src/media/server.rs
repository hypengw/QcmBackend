use futures_util::{SinkExt, Stream, StreamExt};
use http_body_util::{combinators::BoxBody, BodyExt, BodyStream, Full, Limited, StreamBody};
use hyper::body::{Body, Bytes, Frame, Incoming};
use hyper::{Request, Response};
use qcm_core::model as sql_model;
use qcm_core::Result;
use sea_orm::EntityTrait;
use std::sync::Arc;

use crate::error::{HttpError, ProcessError};
use crate::event::BackendContext;
use crate::reverse::body_type::ResponseBody;

pub async fn media_get_image(
    ctx: &Arc<BackendContext>,
    library_id: i64,
    item_id: &str,
    image_type: &str,
) -> Result<Response<ResponseBody>, ProcessError> {
    let library = sql_model::library::Entity::find_by_id(library_id)
        .one(&ctx.provider_context.db)
        .await?
        .ok_or(ProcessError::NoSuchLibrary(library_id.to_string()))?;

    let provider = qcm_core::global::provider(library.provider_id).ok_or(
        ProcessError::NoSuchProvider(library.provider_id.to_string()),
    )?;
    let resp = provider
        .image(&ctx.provider_context, item_id, image_type)
        .await?;

    let status = resp.status();
    let headers = resp.headers().clone();
    let stream = resp.bytes_stream().map(|f| f.map(|b| Frame::data(b)));
    let s = StreamBody::new(stream);

    let builder = {
        let mut builder = Response::builder();
        let dst_headers = builder.headers_mut().unwrap();
        // *dst_headers = headers;
        use reqwest::header;
        for h in [
            header::CONTENT_TYPE,
            header::CONTENT_LENGTH,
            header::CONTENT_DISPOSITION,
            header::AGE,
            header::CACHE_CONTROL,
            header::LAST_MODIFIED,
        ] {
            if let Some(v) = headers.get(&h) {
                dst_headers.insert(h, v.clone());
            }
        }
        builder
    };
    let resp = builder
        .status(status)
        .body(ResponseBody::Boxed(BoxBody::new(s.map_err(|e| e.into()))))
        .unwrap();
    Ok(resp)
}
