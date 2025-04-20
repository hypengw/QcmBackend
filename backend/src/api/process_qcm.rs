use prost::{self, Message};
use qcm_core::provider::{AuthMethod, AuthResult};
use qcm_core::{event::Event as CoreEvent, global, Result};
use std::sync::Arc;
use tokio::sync::oneshot;

use qcm_core::model as sqlm;
use sea_orm::{
    sea_query, ColumnTrait, EntityTrait, LoaderTrait, ModelTrait, PaginatorTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};

use crate::api::{self, pagination::PageParams};
use crate::convert::QcmInto;
use crate::error::ProcessError;
use crate::event::{BackendContext, BackendEvent};
use crate::msg::{
    self, AddProviderRsp, GetAlbumsRsp, GetArtistsRsp, GetProviderMetasRsp, MessageType,
    QcmMessage, QrAuthUrlRsp, Rsp, SyncRsp, TestRsp,
};

fn extra_insert_artists(extra: &mut prost_types::Struct, artists: &[sqlm::artist::Model]) {
    let mut artist_json: Vec<_> = Vec::new();
    for artist in artists {
        artist_json.push(serde_json::json!({
            "id": artist.id.to_string(),
            "name": artist.name,
        }));
    }
    extra.fields.insert(
        "artists".to_string(),
        serde_json::to_string(&artist_json).unwrap().into(),
    );
}

fn extra_insert_album(extra: &mut prost_types::Struct, album: &sqlm::album::Model) {
    let j = serde_json::json!({
        "id": album.id.to_string(),
        "name": album.name,
    });
    extra.fields.insert(
        "album".to_string(),
        serde_json::to_string(&j).unwrap().into(),
    );
}

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
                        let provider = match auth_info.method {
                            Some(msg::model::auth_info::Method::Qr(_)) => {
                                let p = match global::get_tmp_provider() {
                                    Some(tmp) if tmp.type_name() == req.type_name => tmp,
                                    _ => {
                                        let provider =
                                            (meta.creator)(None, "", &global::device_id())?;
                                        global::set_tmp_provider(Some(provider.clone()));
                                        provider
                                    }
                                };
                                p.set_name(&req.name);
                                p
                            }
                            _ => (meta.creator)(None, &req.name, &global::device_id())?,
                        };

                        let mut rsp = AddProviderRsp::default();
                        match provider
                            .auth(&ctx.provider_context, &auth_info.clone().qcm_into())
                            .await?
                        {
                            AuthResult::Ok => {
                                let id = api::db::add_provider(
                                    &ctx.provider_context.db,
                                    provider.clone(),
                                )
                                .await?;
                                provider.set_id(Some(id));

                                global::add_provider(provider.clone());
                                global::set_tmp_provider(None);

                                ctx.bk_ev_sender
                                    .send(BackendEvent::NewProvider { id })
                                    .await?;
                                rsp.code = msg::model::AuthResult::Ok.into();
                            }
                            e => {
                                rsp = e.qcm_into();
                            }
                        };
                        return Ok(rsp.qcm_into());
                    }

                    return Err(ProcessError::NoSuchProviderType(req.type_name.clone()));
                }
                return Err(ProcessError::MissingFields("auth_info".to_string()));
            }
        }
        MessageType::QrAuthUrlReq => {
            if let Some(Payload::QrAuthUrlReq(req)) = payload {
                let provider = {
                    match global::get_tmp_provider() {
                        Some(tmp) if tmp.type_name() == req.provider_meta => tmp,
                        _ => {
                            let meta = global::provider_meta(&req.provider_meta).ok_or(
                                ProcessError::NoSuchProviderType(req.provider_meta.clone()),
                            )?;
                            let provider = (meta.creator)(None, "", &global::device_id())?;
                            global::set_tmp_provider(Some(provider.clone()));
                            provider
                        }
                    }
                };
                let info = provider.qr(&ctx.provider_context).await?;
                let rsp = QrAuthUrlRsp {
                    key: info.key,
                    url: info.url,
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::GetAlbumReq => {
            if let Some(Payload::GetAlbumReq(req)) = payload {
                let db = &ctx.provider_context.db;

                let album = sqlm::album::Entity::find_by_id(req.id)
                    .one(db)
                    .await?
                    .ok_or(ProcessError::NoSuchAlbum(req.id.to_string()))?;

                let artists = album.find_related(sqlm::artist::Entity).all(db).await?;

                let mut songs = Vec::new();
                let mut song_extras = Vec::new();

                for (song, artists) in sqlm::song::Entity::find()
                    .filter(sqlm::song::Column::AlbumId.eq(album.id))
                    .order_by_asc(sqlm::song::Column::TrackNumber)
                    .find_with_related(sqlm::artist::Entity)
                    .all(db)
                    .await?
                {
                    songs.push(song.qcm_into());

                    let mut extra = prost_types::Struct::default();
                    extra_insert_artists(&mut extra, &artists);
                    extra_insert_album(&mut extra, &album);
                    song_extras.push(extra);
                }

                let mut extra = prost_types::Struct::default();
                extra_insert_artists(&mut extra, &artists);

                let rsp = msg::GetAlbumRsp {
                    item: Some(album.qcm_into()),
                    extra: Some(extra),
                    songs,
                    song_extras,
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::GetAlbumsReq => {
            if let Some(Payload::GetAlbumsReq(req)) = payload {
                let page_params = PageParams::new(req.page, req.page_size);

                log::info!("{:?}", &req.library_id);

                let paginator = sqlm::album::Entity::find()
                    .filter(sqlm::album::Column::LibraryId.is_in(req.library_id.clone()))
                    .order_by_asc(sqlm::album::Column::Id)
                    .paginate(&ctx.provider_context.db, page_params.page_size);

                let total = paginator.num_items().await?;
                let albums = paginator.fetch_page(page_params.page).await?;

                let artists = albums
                    .load_many_to_many(
                        sqlm::artist::Entity,
                        sqlm::rel_album_artist::Entity,
                        &ctx.provider_context.db,
                    )
                    .await?;

                let mut items = Vec::new();
                let mut extras = Vec::new();

                for (album, artists) in albums.into_iter().zip(artists.into_iter()) {
                    items.push(album.qcm_into());

                    let mut extra = prost_types::Struct::default();
                    extra_insert_artists(&mut extra, &artists);
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

                let paginator = sqlm::artist::Entity::find()
                    .filter(sqlm::artist::Column::LibraryId.is_in(req.library_id.clone()))
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
        MessageType::GetArtistReq => {
            if let Some(Payload::GetArtistReq(req)) = payload {
                let db = &ctx.provider_context.db;

                let artist = sqlm::artist::Entity::find_by_id(req.id)
                    .one(db)
                    .await?
                    .ok_or(ProcessError::NoSuchArtist(req.id.to_string()))?;

                let rsp = msg::GetArtistRsp {
                    item: Some(artist.qcm_into()),
                    extra: None,
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::GetMixsReq => {
            if let Some(Payload::GetMixsReq(req)) = payload {
                let page_params = PageParams::new(req.page, req.page_size);

                let paginator = sqlm::mix::Entity::find()
                    .paginate(&ctx.provider_context.db, page_params.page_size);

                let total = paginator.num_items().await?;
                let mixes = paginator
                    .fetch_page(page_params.page)
                    .await?
                    .into_iter()
                    .map(|m| m.qcm_into())
                    .collect();

                let rsp = msg::GetMixsRsp {
                    items: mixes,
                    extras: Vec::new(),
                    total: total as i32,
                    has_more: page_params.has_more(total),
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::GetMixReq => {
            if let Some(Payload::GetMixReq(req)) = payload {
                let db = &ctx.provider_context.db;

                let mix = sqlm::mix::Entity::find_by_id(req.id)
                    .one(db)
                    .await?
                    .ok_or(ProcessError::NoSuchMix(req.id.to_string()))?;

                let rsp = msg::GetMixRsp {
                    item: Some(mix.qcm_into()),
                    extra: None,
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::GetArtistAlbumReq => {
            if let Some(Payload::GetArtistAlbumReq(req)) = payload {
                let db = &ctx.provider_context.db;
                let page_params = PageParams::new(req.page, req.page_size);

                let artist = sqlm::artist::Entity::find_by_id(req.id)
                    .one(db)
                    .await?
                    .ok_or(ProcessError::NoSuchArtist(req.id.to_string()))?;

                let albums_query = artist.find_related(sqlm::album::Entity);
                let paginator = albums_query.paginate(db, page_params.page_size);

                let total = paginator.num_items().await?;
                let albums = paginator.fetch_page(page_params.page).await?;

                let artists = albums
                    .load_many_to_many(sqlm::artist::Entity, sqlm::rel_album_artist::Entity, db)
                    .await?;

                let mut items = Vec::new();
                let mut extras = Vec::new();

                for (album, artists) in albums.into_iter().zip(artists.into_iter()) {
                    items.push(album.qcm_into());

                    let mut extra = prost_types::Struct::default();
                    extra_insert_artists(&mut extra, &artists);
                    extras.push(extra);
                }

                let rsp = msg::GetArtistAlbumRsp {
                    items,
                    extras,
                    total: total as i32,
                    has_more: page_params.has_more(total),
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::SyncReq => {
            if let Some(Payload::SyncReq(req)) = payload {
                let (tx, rx) = oneshot::channel::<i64>();
                ctx.provider_context
                    .ev_sender
                    .send(CoreEvent::ProviderSync {
                        id: req.provider_id,
                        oneshot: Some(tx),
                    })
                    .await?;
                let task_id = rx.await;
                let rsp = SyncRsp {
                    handle: task_id.unwrap_or(-1),
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
