use super::process_qcm::process_qcm;
use futures_util::{SinkExt, Stream, StreamExt};
use http_body_util::{combinators::BoxBody, BodyExt, BodyStream, Full, Limited, StreamBody};
use hyper::body::{Body, Bytes, Frame, Incoming};
use hyper::{Request, Response};
pub use hyper_tungstenite::tungstenite::Message as WsMessage;
use prost::{self, Message};
use qcm_core::model as sql_model;
use qcm_core::Result;
use sea_orm::EntityTrait;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use crate::convert::QcmInto;
use crate::error::{HttpError, ProcessError};
use crate::event::BackendContext;
use crate::media::server::media_get_image;
use crate::msg::{self, QcmMessage, Rsp};
use crate::reverse::body_type::ResponseBody;
use qcm_core::anyhow;

const SECURE_MAX_SIZE: usize = 64 * 1024;

pub async fn process_http_post(
    ctx: &Arc<BackendContext>,
    req: Request<Incoming>,
) -> Result<Response<ResponseBody>, ProcessError> {
    let mut in_id: Option<i32> = None;
    let data = Limited::new(req.into_body(), SECURE_MAX_SIZE)
        .collect()
        .await
        .map_err(|e| ProcessError::Internal(anyhow!("{}", e)))?
        .to_bytes();
    let mut qcm_msg = process_qcm(ctx, &data[..], &mut in_id).await;
    if let (Some(id), Ok(qcm_msg)) = (in_id, &mut qcm_msg) {
        qcm_msg.id = id;
    };

    qcm_msg.and_then(|qcm_msg| -> Result<_, ProcessError> {
        let mut buf = Vec::new();
        qcm_msg.encode(&mut buf)?;
        // let stream = futures_util::stream::once(async {
        //     Ok::<_, std::io::Error>(Frame::data(Bytes::from(buf)))
        // });
        Ok(Response::new(ResponseBody::Boxed(
            Full::new(Bytes::from(buf)).map_err(|e| e.into()).boxed(),
        )))
    })
}

pub async fn process_http_get(
    ctx: &Arc<BackendContext>,
    req: Request<Incoming>,
) -> Result<Response<ResponseBody>, ProcessError> {
    let path_segments: Vec<&str> = req
        .uri()
        .path()
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    match path_segments.as_slice() {
        ["image", library_id, item_id, image_id] => {
            media_get_image(
                ctx,
                library_id
                    .parse()
                    .map_err(|_| ProcessError::NoSuchLibrary(library_id.to_string()))?,
                item_id,
                &image_id,
            )
            .await
        }
        _ => {
            // 404
            Ok(Response::new(ResponseBody::Boxed(
                Full::new(Bytes::from("404 Not Found"))
                    .map_err(|e| e.into())
                    .boxed(),
            )))
        }
    }
}
