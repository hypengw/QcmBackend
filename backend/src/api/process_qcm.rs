use prost::{self, Message};
use qcm_core::{global, Result};
use std::sync::Arc;

use qcm_core::model as sql_model;
use sea_orm::{sea_query, EntityTrait, LoaderTrait, PaginatorTrait};

use crate::api::{self, pagination::PageParams};
use crate::convert::QcmInto;
use crate::error::ProcessError;
use crate::event::{BackendContext, BackendEvent};
use crate::msg::{
    self, GetAlbumsRsp, GetArtistsRsp, GetProviderMetasRsp, MessageType, QcmMessage, Rsp, TestRsp,
};

pub async fn process_qcm(
    ctx: &Arc<BackendContext>,
    data: &[u8],
    in_id: &mut Option<i32>,
) -> Result<QcmMessage, ProcessError> {
    use msg::qcm_message::Payload;
    let message = QcmMessage::decode(data)?;
    let mtype = message.r#type();
    let payload = &message.payload;
    *in_id = Some(message.id);

    match mtype {
        MessageType::GetProviderMetasReq => {
            let response = GetProviderMetasRsp {
                metas: qcm_core::global::with_provider_metas(|metas| {
                    metas
                        .values()
                        .map(|el| -> msg::model::ProviderMeta { el.clone().qcm_into() })
                        .collect()
                }),
            };
            return Ok(response.qcm_into());
        }
        MessageType::AddProviderReq => {
            if let Some(Payload::AddProviderReq(req)) = payload {
                if let Some(auth_info) = &req.auth_info {
                    if let Some(meta) = qcm_core::global::provider_meta(&req.type_name) {
                        let provider = (meta.creator)(None, &req.name, &global::device_id());
                        provider
                            .login(&ctx.provider_context, &auth_info.clone().qcm_into())
                            .await?;

                        let id = api::db::add_provider(&ctx.provider_context.db, provider.clone())
                            .await?;
                        provider.set_id(Some(id));

                        global::add_provider(provider.clone());

                        ctx.bk_ev_sender
                            .send(BackendEvent::NewProvider { id })
                            .await?;
                        return Ok(Rsp::default().qcm_into());
                    }
                    return Err(ProcessError::NoSuchProviderType(req.type_name.clone()));
                }
                return Err(ProcessError::MissingFields("auth_info".to_string()));
            }
        }
        MessageType::GetAlbumsReq => {
            if let Some(Payload::GetAlbumsReq(req)) = payload {
                let page_params = PageParams::new(req.page, req.page_size);

                let paginator = sql_model::album::Entity::find()
                    .paginate(&ctx.provider_context.db, page_params.page_size);

                let total = paginator.num_items().await?;
                let albums = paginator.fetch_page(page_params.page).await?;

                let artists = albums
                    .load_many_to_many(
                        sql_model::artist::Entity,
                        sql_model::rel_album_artist::Entity,
                        &ctx.provider_context.db,
                    )
                    .await?;

                let mut items = Vec::new();
                let mut extras = Vec::new();

                for (album, artists) in albums.into_iter().zip(artists.into_iter()) {
                    items.push(album.qcm_into());

                    let mut extra = prost_types::Struct::default();
                    let mut artist_json = serde_json::Value::Array(vec![]);
                    for artist in artists {
                        artist_json.as_array_mut().unwrap().insert(
                            0,
                            serde_json::json!({
                                "id": artist.id.to_string(),
                                "name": artist.name,
                            }),
                        );
                    }
                    extra
                        .fields
                        .insert("artists".to_string(), artist_json.to_string().into());

                    extras.push(extra);
                }

                let rsp = GetAlbumsRsp {
                    items,
                    extras,
                    total: total as i32,
                    has_more: page_params.has_more(total),
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::GetArtistsReq => {
            if let Some(Payload::GetArtistsReq(req)) = payload {
                let page_params = PageParams::new(req.page, req.page_size);

                let paginator = sql_model::artist::Entity::find()
                    .paginate(&ctx.provider_context.db, page_params.page_size);

                let total = paginator.num_items().await?;
                let artists = paginator
                    .fetch_page(page_params.page)
                    .await?
                    .into_iter()
                    .map(|a| a.qcm_into())
                    .collect();

                let rsp = GetArtistsRsp {
                    items: artists,
                    extras: Vec::new(),
                    total: total as i32,
                    has_more: page_params.has_more(total),
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::TestReq => {
            if let Some(Payload::TestReq(req)) = payload {
                let rsp = TestRsp {
                    test_data: format!("Echo: {}", req.test_data),
                };
                return Ok(rsp.qcm_into());
            }
        }
        _ => {
            return Err(ProcessError::UnsupportedMessageType(mtype.into()));
        }
    }
    return Err(ProcessError::UnexpectedPayload(mtype.into()));
}
