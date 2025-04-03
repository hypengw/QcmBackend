use super::process_qcm::process_qcm;
use futures_util::{SinkExt, Stream, StreamExt};
use http_body_util::{combinators::BoxBody, BodyExt, BodyStream, Full, Limited, StreamBody};
use hyper::body::{Body, Bytes, Frame, Incoming};
use hyper::{Request, Response, StatusCode};
pub use hyper_tungstenite::tungstenite::Message as WsMessage;
use prost::{self, Message};
use qcm_core::model::type_enum::ImageType;
use qcm_core::{anyhow, model as sqlm, model::type_enum::ItemType, Result};
use sea_orm::{EntityTrait, FromQueryResult, QuerySelect, QueryTrait};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use crate::convert::QcmInto;
use crate::error::{HttpError, ProcessError};
use crate::event::BackendContext;
use crate::media::server::{media_get_audio, media_get_image};
use crate::msg::{self, QcmMessage, Rsp};
use crate::reverse::body_type::ResponseBody;

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
    log::warn!("url: {}", req.uri());
    let path_segments: Vec<&str> = req.uri().path().split('/').skip(1).collect();

    let parse_id = |id: &str| -> Result<i64, ProcessError> {
        return id
            .parse()
            .map_err(|_| ProcessError::WrongId(id.to_string()));
    };
    let db = &ctx.provider_context.db;

    match path_segments.as_slice() {
        ["image", item_type, id, image_type] => {
            let image_type = ImageType::from_str(&image_type)
                .map_err(|_| ProcessError::NoSuchImageType(image_type.to_string()))?;
            match ItemType::from_str(&item_type)
                .map_err(|_| ProcessError::NoSuchItemType(item_type.to_string()))?
            {
                ItemType::Album => {
                    let id = parse_id(id)?;
                    let (native_id, provider_id): (String, i64) =
                        sqlm::album::Entity::find_by_id(id)
                            .select_only()
                            .column(sqlm::album::Column::NativeId)
                            .column(sqlm::library::Column::ProviderId)
                            .left_join(sqlm::library::Entity)
                            .into_tuple()
                            .one(db)
                            .await?
                            .ok_or(ProcessError::NoSuchAlbum(id.to_string()))?;

                    media_get_image(ctx, provider_id, &native_id, image_type).await
                }
                _ => Err(ProcessError::UnsupportedItemType(item_type.to_string())),
            }
        }
        ["audio", item_type, id] => {
            match ItemType::from_str(&item_type)
                .map_err(|_| ProcessError::NoSuchItemType(item_type.to_string()))?
            {
                ItemType::Song => {
                    let id = parse_id(id)?;
                    let (native_id, provider_id): (String, i64) =
                        sqlm::song::Entity::find_by_id(id)
                            .select_only()
                            .column(sqlm::album::Column::NativeId)
                            .column(sqlm::library::Column::ProviderId)
                            .left_join(sqlm::library::Entity)
                            .into_tuple()
                            .one(db)
                            .await?
                            .ok_or(ProcessError::NoSuchSong(id.to_string()))?;

                    media_get_audio(ctx, provider_id, &native_id).await
                }
                _ => Err(ProcessError::UnsupportedItemType(item_type.to_string())),
            }
        }
        _ => {
            let rsp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(ResponseBody::Empty)
                .unwrap();
            Ok(rsp)
        }
    }
}
