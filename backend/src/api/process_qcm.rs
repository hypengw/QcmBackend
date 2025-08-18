use migration::query;
use prost::{self, Message};
use qcm_core::db::values::Timestamp;
use qcm_core::provider::{AuthMethod, AuthResult};
use qcm_core::{event::Event as CoreEvent, global, Result};
use sea_orm::ActiveValue::NotSet;
use std::sync::Arc;
use tokio::sync::oneshot;

use qcm_core::model::{self as sqlm, dynamic, provider};
use sea_orm::{
    sea_query, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityName, EntityTrait,
    FromQueryResult, LoaderTrait, ModelTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    Statement,
};

use crate::api::{self, pagination::PageParams};
use crate::convert::QcmInto;
use crate::db::filter::SelectQcmMsgFilters;
use crate::error::ProcessError;
use crate::event::{BackendContext, BackendEvent};
use crate::msg::{
    self, AuthProviderRsp, GetAlbumArtistsRsp, GetAlbumsRsp, GetArtistsRsp, GetProviderMetasRsp,
    GetSongsRsp, MessageType, QcmMessage, QrAuthUrlRsp, Rsp, SyncRsp, TestRsp,
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

fn extra_insert_dynamic(extra: &mut prost_types::Struct, dy: &sqlm::dynamic::Model) {
    let j = serde_json::json!({
        "is_favorite": dy.favorite_at.is_some()
    });
    extra.fields.insert(
        "dynamic".to_string(),
        serde_json::to_string(&j).unwrap().into(),
    );
}

async fn to_rsp_songs(
    db: &DatabaseConnection,
    songs: Vec<sqlm::song::Model>,
    album: Option<&sqlm::album::Model>,
) -> Result<(Vec<msg::model::Song>, Vec<prost_types::Struct>), ProcessError> {
    let artists = songs
        .load_many_to_many(sqlm::artist::Entity, sqlm::rel_song_artist::Entity, db)
        .await?;

    let dynamics = songs.load_one(sqlm::dynamic::Entity, db).await?;

    let mut items = Vec::new();
    let mut extras = Vec::new();

    for ((song, artists), dy) in songs.into_iter().zip(artists.into_iter()).zip(dynamics) {
        items.push(song.qcm_into());
        let mut extra = prost_types::Struct::default();
        extra_insert_artists(&mut extra, &artists);
        if let Some(dy) = dy {
            extra_insert_dynamic(&mut extra, &dy);
        }
        if let Some(album) = &album {
            extra_insert_album(&mut extra, &album);
        }
        extras.push(extra);
    }
    Ok((items, extras))
}

async fn to_rsp_albums(
    db: &DatabaseConnection,
    albums: Vec<sqlm::album::Model>,
) -> Result<(Vec<msg::model::Album>, Vec<prost_types::Struct>), ProcessError> {
    let artists = albums
        .load_many_to_many(sqlm::artist::Entity, sqlm::rel_album_artist::Entity, db)
        .await?;

    let dynamics = albums.load_one(sqlm::dynamic::Entity, db).await?;

    let mut items = Vec::new();
    let mut extras = Vec::new();

    for ((album, artists), dy) in albums.into_iter().zip(artists.into_iter()).zip(dynamics) {
        items.push(album.qcm_into());
        let mut extra = prost_types::Struct::default();
        extra_insert_artists(&mut extra, &artists);
        if let Some(dy) = dy {
            extra_insert_dynamic(&mut extra, &dy);
        }
        extras.push(extra);
    }
    Ok((items, extras))
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
                let id =
                    crate::db::add_provider(&ctx.provider_context.db, provider.clone()).await?;
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
                // maybe: try use tmp self?
                tmp_provider.set_name(&_provider.name());

                // replace
                crate::db::add_provider(&ctx.provider_context.db, tmp_provider.clone()).await?;
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
                                crate::db::add_provider(
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
                        crate::db::add_provider(&ctx.provider_context.db, provider).await?;
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

                let (album, dy) = sqlm::album::Entity::find_by_id(req.id)
                    .find_also_related(sqlm::dynamic::Entity)
                    .one(db)
                    .await?
                    .ok_or(ProcessError::NoSuchAlbum(req.id.to_string()))?;

                let artists = album.find_related(sqlm::artist::Entity).all(db).await?;

                let (songs, song_extras) = {
                    let songs = sqlm::song::Entity::find()
                        .filter(sqlm::song::Column::AlbumId.eq(album.id))
                        .order_by_asc(sqlm::song::Column::TrackNumber)
                        .all(db)
                        .await?;

                    to_rsp_songs(db, songs, Some(&album)).await?
                };

                let mut extra = prost_types::Struct::default();
                extra_insert_artists(&mut extra, &artists);
                if let Some(dy) = dy {
                    extra_insert_dynamic(&mut extra, &dy);
                }

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

                let sort: msg::model::AlbumSort =
                    req.sort.try_into().unwrap_or(msg::model::AlbumSort::Title);
                let sort_col: sqlm::album::Column = sort.qcm_into();
                let query = sqlm::album::Entity::find()
                    .left_join(sqlm::dynamic::Entity)
                    .filter(sqlm::album::Column::LibraryId.is_in(req.library_id.clone()))
                    .filter(sqlm::dynamic::Column::IsExternal.eq(false))
                    .qcm_filters(&req.filters)
                    .order_by(
                        sort_col,
                        match req.sort_asc {
                            true => sea_orm::Order::Asc,
                            false => sea_orm::Order::Desc,
                        },
                    );

                let paginator = query.paginate(&ctx.provider_context.db, page_params.page_size);

                let total = paginator.num_items().await?;
                let albums = paginator.fetch_page(page_params.page).await?;

                let (items, extras) = to_rsp_albums(&ctx.provider_context.db, albums).await?;

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

                let sort: msg::model::ArtistSort =
                    req.sort.try_into().unwrap_or(msg::model::ArtistSort::Name);
                let sort_col: sqlm::artist::Column = sort.qcm_into();
                let paginator = sqlm::artist::Entity::find()
                    .filter(sqlm::artist::Column::LibraryId.is_in(req.library_id.clone()))
                    .inner_join(sqlm::rel_album_artist::Entity)
                    .qcm_filters(&req.filters)
                    .order_by(
                        sort_col,
                        match req.sort_asc {
                            true => sea_orm::Order::Asc,
                            false => sea_orm::Order::Desc,
                        },
                    )
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
                let sort: msg::model::ArtistSort =
                    req.sort.try_into().unwrap_or(msg::model::ArtistSort::Name);
                let sort_col: sqlm::artist::Column = sort.qcm_into();
                let paginator = sqlm::artist::Entity::find()
                    .filter(sqlm::artist::Column::LibraryId.is_in(req.library_id.clone()))
                    .inner_join(sqlm::rel_song_artist::Entity)
                    .qcm_filters(&req.filters)
                    .order_by(
                        sort_col,
                        match req.sort_asc {
                            true => sea_orm::Order::Asc,
                            false => sea_orm::Order::Desc,
                        },
                    )
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
                let sort: msg::model::AlbumSort = req
                    .sort
                    .try_into()
                    .unwrap_or(msg::model::AlbumSort::PublishTime);
                let sort_col: sqlm::album::Column = sort.qcm_into();

                let artist = sqlm::artist::Entity::find_by_id(req.id)
                    .one(db)
                    .await?
                    .ok_or(ProcessError::NoSuchArtist(req.id.to_string()))?;

                let albums_query = artist.find_related(sqlm::album::Entity).order_by(
                    sort_col,
                    match req.sort_asc {
                        true => sea_orm::Order::Asc,
                        false => sea_orm::Order::Desc,
                    },
                );
                let paginator = albums_query.paginate(db, page_params.page_size);

                let total = paginator.num_items().await?;
                let albums = paginator.fetch_page(page_params.page).await?;

                let (items, extras) = to_rsp_albums(db, albums).await?;

                let rsp = msg::GetArtistAlbumRsp {
                    items,
                    extras,
                    total: total as i32,
                    has_more: page_params.has_more(total),
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::GetSubtitleReq => {
            if let Some(Payload::GetSubtitleReq(req)) = payload {
                let db = &ctx.provider_context.db;

                let (native_id, provider_id): (String, i64) =
                    sqlm::song::Entity::find_by_id(req.song_id)
                        .select_only()
                        .column(sqlm::song::Column::NativeId)
                        .column(sqlm::library::Column::ProviderId)
                        .left_join(sqlm::library::Entity)
                        .into_tuple()
                        .one(db)
                        .await?
                        .ok_or(ProcessError::NoSuchSong(req.song_id.to_string()))?;

                let provider = global::provider(provider_id)
                    .ok_or(ProcessError::NoSuchProvider(provider_id.to_string()))?;

                let subtitle = provider.subtitle(&native_id).await?;
                let rsp = msg::GetSubtitleRsp {
                    subtitle: Some(subtitle.qcm_into()),
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::SearchReq => {
            if let Some(Payload::SearchReq(req)) = payload {
                let db = &ctx.provider_context.db;
                let page_params = PageParams::new(req.page, req.page_size);
                let search_query = req.query.clone();
                let mut albums_rsp = None;
                let mut songs_rsp = None;
                let mut artists_rsp = None;

                let library_ids = if search_query.is_empty() {
                    String::new()
                } else {
                    req.library_id
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                };

                let format_query = |table: &str, fts: &str| {
                    let db_backend = ctx.provider_context.db.get_database_backend();
                    Statement::from_sql_and_values(
                        db_backend,
                        format!(
                            r#"
                                    SELECT {table}.* FROM {table}
                                    INNER JOIN {fts} ON {table}.id = {fts}.rowid
                                    WHERE {fts} MATCH qcm_query(?) AND {table}.library_id IN ({library_ids})
                                    "#
                        ),
                        [search_query.clone().into()],
                    )
                };

                for search_type in &req.types {
                    let search_type: msg::SearchType =
                        msg::SearchType::try_from(search_type.clone())
                            .map_err(|_| ProcessError::NoSuchSearchType(search_type.to_string()))?;
                    match search_type {
                        msg::SearchType::Album => {
                            let entity = sqlm::album::Entity::default();
                            let table = entity.table_name();
                            let fts = format!("{table}_fts");
                            let albums_query =
                                sqlm::album::Entity::find().from_raw_sql(format_query(table, &fts));

                            let paginator = albums_query.paginate(db, page_params.page_size);
                            let total = paginator.num_items().await?;
                            let albums = paginator.fetch_page(page_params.page).await?;

                            let (items, extras) = to_rsp_albums(db, albums).await?;

                            albums_rsp = Some(GetAlbumsRsp {
                                items,
                                extras,
                                total: total as i32,
                                has_more: page_params.has_more(total),
                            });
                        }
                        msg::SearchType::Song => {
                            let entity = sqlm::song::Entity::default();
                            let table = entity.table_name();
                            let fts = format!("{table}_fts");
                            let query =
                                sqlm::song::Entity::find().from_raw_sql(format_query(table, &fts));

                            let paginator = query.paginate(db, page_params.page_size);
                            let total = paginator.num_items().await?;
                            let songs = paginator.fetch_page(page_params.page).await?;

                            let (items, extras) = to_rsp_songs(db, songs, None).await?;

                            songs_rsp = Some(GetSongsRsp {
                                items,
                                extras,
                                total: total as i32,
                                has_more: page_params.has_more(total),
                            });
                        }
                        msg::SearchType::Artist => {
                            let entity = sqlm::artist::Entity::default();
                            let table = entity.table_name();
                            let fts = format!("{table}_fts");
                            let query = sqlm::artist::Entity::find()
                                .from_raw_sql(format_query(table, &fts));
                            let page_params = PageParams::new(req.page, req.page_size);

                            let paginator = query.paginate(db, page_params.page_size);
                            let total = paginator.num_items().await?;

                            let artists = paginator
                                .fetch_page(page_params.page)
                                .await?
                                .into_iter()
                                .map(|a| a.qcm_into())
                                .collect();

                            artists_rsp = Some(GetArtistsRsp {
                                items: artists,
                                extras: Vec::new(),
                                total: total as i32,
                                has_more: page_params.has_more(total),
                            });
                        }
                    }
                }

                let rsp = msg::SearchRsp {
                    albums: albums_rsp,
                    artists: artists_rsp,
                    songs: songs_rsp,
                };
                return Ok(rsp.qcm_into());
            }
        }
        MessageType::SetFavoriteReq => {
            if let Some(Payload::SetFavoriteReq(req)) = payload {
                let db = &ctx.provider_context.db;
                use sqlm::type_enum::ItemType;
                let item_type: ItemType = req
                    .item_type
                    .try_into()
                    .map_err(|_| ProcessError::NoSuchItemType(req.item_type.to_string()))?;

                use sea_orm::{sea_query::Expr, Set};

                let (library_id, provider_id, native_id): (i64, i64, String) = {
                    let query = sqlm::library::Entity::find()
                        .select_only()
                        .column(sqlm::library::Column::LibraryId)
                        .column(sqlm::library::Column::ProviderId);
                    match item_type {
                        ItemType::Album => query
                            .column(sqlm::album::Column::NativeId)
                            .inner_join(sqlm::album::Entity)
                            .filter(
                                Expr::col((sqlm::album::Entity, sqlm::album::Column::Id))
                                    .eq(req.id),
                            ),
                        ItemType::Artist => query
                            .column(sqlm::artist::Column::NativeId)
                            .inner_join(sqlm::artist::Entity)
                            .filter(
                                Expr::col((sqlm::artist::Entity, sqlm::artist::Column::Id))
                                    .eq(req.id),
                            ),
                        ItemType::Song => query
                            .column(sqlm::song::Column::NativeId)
                            .inner_join(sqlm::song::Entity)
                            .filter(
                                Expr::col((sqlm::song::Entity, sqlm::song::Column::Id)).eq(req.id),
                            ),
                        _ => query,
                    }
                    .into_tuple()
                    .one(db)
                    .await?
                    .ok_or(ProcessError::NotFound)?
                };

                let provider = global::provider(provider_id)
                    .ok_or(ProcessError::NoSuchProvider(provider_id.to_string()))?;

                provider
                    .favorite(&ctx.provider_context, &native_id, item_type, req.value)
                    .await?;

                match item_type {
                    ItemType::Album | ItemType::Song | ItemType::Artist => {
                        let m = sqlm::dynamic::ActiveModel {
                            id: NotSet,
                            item_id: Set(req.id),
                            item_type: Set(item_type.into()),
                            favorite_at: Set(match req.value {
                                true => Some(Timestamp::now()),
                                false => None,
                            }),
                            library_id: Set(library_id),
                            ..Default::default()
                        };

                        sqlm::dynamic::Entity::insert(m)
                            .on_conflict(
                                sea_query::OnConflict::columns([
                                    sqlm::dynamic::Column::ItemId,
                                    sqlm::dynamic::Column::ItemType,
                                ])
                                .update_column(sqlm::dynamic::Column::FavoriteAt)
                                .to_owned(),
                            )
                            .exec(db)
                            .await?;
                    }
                    _ => {}
                }
                return Ok(Rsp::default().qcm_into());
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
