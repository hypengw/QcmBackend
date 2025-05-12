use prost::{self, Message};
use qcm_core::provider::{AuthMethod, AuthResult};
use qcm_core::{event::Event as CoreEvent, global, Result};
use std::sync::Arc;
use tokio::sync::oneshot;

use qcm_core::model as sqlm;
use sea_orm::{
    sea_query, ColumnTrait, ConnectionTrait, EntityTrait, LoaderTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, QueryTrait, TransactionTrait,
};

use crate::api::{self, pagination::PageParams};
use crate::convert::QcmInto;
use crate::error::ProcessError;
use crate::event::{BackendContext, BackendEvent};
use crate::msg::{
    self, AuthProviderRsp, GetAlbumArtistsRsp, GetAlbumsRsp, GetArtistsRsp, GetProviderMetasRsp,
    MessageType, QcmMessage, QrAuthUrlRsp, Rsp, SyncRsp, TestRsp,
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
        MessageType::DeleteProviderReq => {
            if let Some(Payload::DeleteProviderReq(req)) = payload {
                let id = req.provider_id;
                sqlm::provider::Entity::delete_by_id(id)
                    .exec(&ctx.provider_context.db)
                    .await?;
                global::remove_provider(id);
                let _ = ctx
                    .backend_ev
                    .send(BackendEvent::DeleteProvider { id })
                    .await;
                let rsp = Rsp::default();
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::AuthProviderReq => {
            if let Some(Payload::AuthProviderReq(req)) = payload {
                let provider = global::get_tmp_provider(&req.tmp_provider)
                    .ok_or(ProcessError::NoSuchProvider(req.tmp_provider.clone()))?;

                let mut rsp = AuthProviderRsp::default();
                let info = req
                    .auth_info
                    .clone()
                    .ok_or(ProcessError::NotFound)?
                    .qcm_into();
                match provider.auth(&ctx.provider_context, &info).await? {
                    AuthResult::Ok => {
                        rsp.code = msg::model::AuthResult::Ok.into();
                    }
                    e => {
                        rsp = e.qcm_into();
                    }
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::AddProviderReq => {
            if let Some(Payload::AddProviderReq(req)) = payload {
                let provider = global::get_tmp_provider(&req.tmp_provider)
                    .ok_or(ProcessError::NoSuchProvider(req.tmp_provider.clone()))?;
                provider.set_name(&req.name);
                let id = api::db::add_provider(&ctx.provider_context.db, provider.clone()).await?;
                provider.set_id(Some(id));

                global::add_provider(provider.clone());
                ctx.backend_ev
                    .send(BackendEvent::NewProvider { id })
                    .await?;

                return Ok(Rsp::default().qcm_into());
            }
        }
        MessageType::ReplaceProviderReq => {
            if let Some(Payload::ReplaceProviderReq(req)) = payload {
                let _provider = global::provider(req.provider_id)
                    .ok_or(ProcessError::NoSuchProvider(req.provider_id.to_string()))?;
                let tmp_provider = global::get_tmp_provider(&req.tmp_provider)
                    .ok_or(ProcessError::NoSuchProvider(req.tmp_provider.clone()))?;
                tmp_provider.set_id(Some(req.provider_id));
                // replace
                global::add_provider(tmp_provider.clone());
                return Ok(Rsp::default().qcm_into());
            }
        }
        MessageType::UpdateProviderReq => {
            if let Some(Payload::UpdateProviderReq(req)) = payload {
                let provider = global::provider(req.provider_id)
                    .ok_or(ProcessError::NoSuchProvider(req.provider_id.to_string()))?;

                let mut rsp = msg::UpdateProviderRsp::default();
                rsp.code = msg::model::AuthResult::Ok.into();
                match &req.auth_info {
                    // do replace
                    Some(auth_info) => {
                        let type_name = provider.type_name();
                        let meta = global::provider_meta(&type_name)
                            .ok_or(ProcessError::NoSuchProviderType(type_name))?;
                        let new_provider =
                            (meta.creator)(provider.id(), &req.name, &global::device_id())?;

                        let info = auth_info.clone().qcm_into();
                        match new_provider.auth(&ctx.provider_context, &info).await? {
                            AuthResult::Ok => {
                                super::db::add_provider(
                                    &ctx.provider_context.db,
                                    new_provider.clone(),
                                )
                                .await?;
                                global::add_provider(new_provider);
                            }
                            e => {
                                rsp = e.qcm_into();
                            }
                        }
                    }
                    // do update
                    None => {
                        provider.set_name(&req.name);
                        super::db::add_provider(&ctx.provider_context.db, provider).await?;
                    }
                }
                let _ = ctx
                    .backend_ev
                    .send(BackendEvent::UpdateProvider {
                        id: req.provider_id,
                    })
                    .await;
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::CreateTmpProviderReq => {
            if let Some(Payload::CreateTmpProviderReq(req)) = payload {
                let meta = global::provider_meta(&req.type_name)
                    .ok_or(ProcessError::NoSuchProviderType(req.type_name.clone()))?;
                let provider = (meta.creator)(None, "", &global::device_id())?;
                let key = uuid::Uuid::new_v4().to_string();
                global::set_tmp_provider(&key, provider.clone());

                let rsp = msg::CreateTmpProviderRsp { key: key };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::DeleteTmpProviderReq => {
            if let Some(Payload::DeleteTmpProviderReq(req)) = payload {
                global::remove_tmp_provider(&req.key);
            }
        }
        MessageType::QrAuthUrlReq => {
            if let Some(Payload::QrAuthUrlReq(req)) = payload {
                let provider = {
                    match global::get_tmp_provider(&req.tmp_provider) {
                        Some(tmp) => tmp,
                        _ => {
                            return Err(ProcessError::NoSuchProvider(req.tmp_provider.clone()));
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
        MessageType::GetAlbumArtistsReq => {
            if let Some(Payload::GetAlbumArtistsReq(req)) = payload {
                let page_params = PageParams::new(req.page, req.page_size);

                let paginator = sqlm::artist::Entity::find()
                    .filter(sqlm::artist::Column::LibraryId.is_in(req.library_id.clone()))
                    .inner_join(sqlm::rel_album_artist::Entity)
                    .order_by_asc(sqlm::artist::Column::Id)
                    .distinct()
                    .paginate(&ctx.provider_context.db, page_params.page_size);

                let total = paginator.num_items().await?;
                let artists = paginator
                    .fetch_page(page_params.page)
                    .await?
                    .into_iter()
                    .map(|a| a.qcm_into())
                    .collect();

                let rsp = GetAlbumArtistsRsp {
                    items: artists,
                    extras: Vec::new(),
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
                    .inner_join(sqlm::rel_song_artist::Entity)
                    .order_by_asc(sqlm::artist::Column::Id)
                    .distinct()
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
