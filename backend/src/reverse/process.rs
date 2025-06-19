use bytes::BufMut;
use bytes::{Buf, Bytes};
use futures::channel::mpsc::Sender as BoundedSender;
use futures::{SinkExt, StreamExt};
use http_body_util::StreamBody;
use hyper_util::client::legacy::connect::Connected;
use qcm_core::model as sqlm;
use qcm_core::model::type_enum::CacheType;
use qcm_core::Result;
use sea_orm::{sea_query, QuerySelect};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter};

use super::body_type::{self, ResponseBody, StreamItem};
use super::connection::{
    default_range, parse_content_range, range_start, Connection, ContentRange, Range,
    RemoteFileInfo,
};
use super::io::ReadState;
use super::piece;
use crate::error::{HttpError, ProcessError};
use crate::reverse::connection::{format_range, range_in_full};
use crate::reverse::io::IoContext;
use reqwest::Response;
use std::collections::BTreeMap;
use std::future::Future;
use std::io::Read;
use std::io::{Seek, Write};
use std::ops::Deref;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

type Creator = Box<
    dyn Fn(
            bool,
            Option<Range>,
        ) -> Pin<Box<dyn Future<Output = Result<Response, ProcessError>> + Send>>
        + Send
        + Sync
        + 'static,
>;

enum EventBus {
    RequestRead(String, i64, u64),
    RequestReadIoRsp(i64, Option<piece::Piece>),
    DoRealRequest(i64),
    ReadedBuf(i64, Bytes, ReadState),
    ReadContinue(i64),
    DoRead,
    NewRemoteFile(String, i64, CacheType, RemoteFileInfo, reqwest::Response),
    FinishFile(String, CacheType, RemoteFileInfo),
    NewConnection(i64, Sender<ConnectionEvent>),
    EndConnection(i64),
}

pub enum ReverseEvent {
    NewConnection(
        Connection,
        Creator,
        oneshot::Sender<Result<hyper::Response<ResponseBody>, hyper::Error>>,
    ),
    NewRemoteFile(String, i64, CacheType, RemoteFileInfo, reqwest::Response),
    RequestReadIoRsp(i64, Option<piece::Piece>),
    FinishFile(String, CacheType, RemoteFileInfo),
    EndConnection(i64),
    Stop,
}

enum ConnectionEvent {
    ReadedBuf(Bytes, ReadState),
    RealRequest,
}

enum IoEvent {
    RequestRead(String, i64, u64),
    DoRead,
    ReadContinue(i64),
    NewWrite(
        String,
        u64,
        CacheType,
        RemoteFileInfo,
        oneshot::Sender<bool>,
    ),
    DoWrite(Arc<String>, u64, Bytes),
    EndConnection(i64),
    EndWrite(String),
}

pub fn wrap_creator<Fut>(ct: impl Fn(bool, Option<Range>) -> Fut + Send + Sync + 'static) -> Creator
where
    Fut: Future<Output = Result<Response, ProcessError>> + Send + 'static,
{
    Box::new(move |head: bool, r: Option<Range>| Box::pin(ct(head, r)))
}

async fn query_cache_info(db: &DatabaseConnection, key: &str) -> Option<sqlm::cache::Model> {
    use sea_orm::ColumnTrait;
    match sqlm::cache::Entity::find()
        .filter(sqlm::cache::Column::Key.eq(key.to_string()))
        .one(db)
        .await
    {
        Ok(info) => info,
        Err(e) => {
            log::error!("{:?}", e);
            None
        }
    }
}

fn cache_to_remote_info(cache_info: &sqlm::cache::Model, range: &Option<Range>) -> RemoteFileInfo {
    let full = cache_info.content_length as u64;
    match range {
        Some(range) => RemoteFileInfo {
            content_length: full - range_start(range, full),
            content_type: cache_info.content_type.clone(),
            accept_ranges: true,
            content_range: ContentRange::from_range(
                range.clone(),
                cache_info.content_length as u64,
            ),
        },
        None => RemoteFileInfo {
            content_length: full,
            content_type: cache_info.content_type.clone(),
            accept_ranges: true,
            content_range: None,
        },
    }
}

fn create_rsp(
    id: i64,
    range: &Option<Range>,
    info: &RemoteFileInfo,
) -> (
    hyper::Response<ResponseBody>,
    BoundedSender<body_type::StreamItem>,
) {
    let (stream_tx, stream_rx) = futures::channel::mpsc::channel(4);
    let rsp = match (range, &info.content_range) {
        (Some(r), Some(cr)) => {
            if range_in_full(&r, cr.full) {
                log::debug!(target: "reverse", "rsp({}) range: {}, content range: {}", id, format_range(&r), cr);
                hyper::Response::builder()
                    .status(hyper::StatusCode::PARTIAL_CONTENT)
                    .header(hyper::header::CONTENT_TYPE, &info.content_type)
                    .header(hyper::header::CONTENT_LENGTH, info.content_length)
                    .header(hyper::header::ACCEPT_RANGES, "bytes")
                    .header(hyper::header::CONTENT_RANGE, cr.to_string())
                    .body(ResponseBody::BoundedStreamed(StreamBody::new(stream_rx)))
                    .unwrap()
            } else {
                log::debug!(target: "reverse", "rsp({}) range not satisfiable", id);
                hyper::Response::builder()
                    .status(hyper::StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(hyper::header::CONTENT_TYPE, &info.content_type)
                    .header(hyper::header::ACCEPT_RANGES, "bytes")
                    .header(hyper::header::CONTENT_RANGE, format!("bytes */{}", cr.full))
                    .body(ResponseBody::Empty)
                    .unwrap()
            }
        }
        (_, _) => {
            log::debug!(target: "reverse", "rsp({}) no range", id);
            let mut b = hyper::Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, &info.content_type)
                .header(hyper::header::CONTENT_LENGTH, info.content_length);

            if info.accept_ranges {
                b = b.header(hyper::header::ACCEPT_RANGES, "bytes");
            }

            b.body(ResponseBody::BoundedStreamed(StreamBody::new(stream_rx)))
                .unwrap()
        }
    };
    (rsp, stream_tx)
}

struct ConnectResource {
    pub tx: Sender<ConnectionEvent>,
}

struct RemoteFileTask {
    pub cursor: Arc<AtomicU64>,
    pub end: u64,
    pub handle: tokio::task::JoinHandle<()>,
}
struct ProcessCtx {
    pub tasks: BTreeMap<i64, RemoteFileTask>,
}

async fn real_request(
    ct: &Creator,
    head: bool,
    range: Option<Range>,
) -> Result<(RemoteFileInfo, reqwest::Response), ProcessError> {
    let rsp = ct(head, range).await;
    match rsp {
        Ok(rsp) => {
            let headers = rsp.headers();
            let content_type = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string());
            let content_length: Option<u64> = headers
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            let content_range = headers
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| parse_content_range(v));

            let accept_ranges = {
                if content_range.is_some() {
                    true
                } else {
                    headers
                        .get(reqwest::header::ACCEPT_RANGES)
                        .map(|v| v == "bytes")
                        .unwrap_or(false)
                }
            };

            match (content_type, content_length) {
                (Some(content_type), Some(content_length)) => Ok((
                    RemoteFileInfo {
                        content_type,
                        content_length,
                        accept_ranges,
                        content_range,
                    },
                    rsp,
                )),
                _ => Err(qcm_core::anyhow!("content_type/content_length not valid").into()),
            }
        }
        Err(e) => Err(e),
    }
}

async fn new_remote_file(
    key: String,
    cache_type: CacheType,
    info: RemoteFileInfo,
    rsp: Response,
    io_tx: std::sync::mpsc::Sender<IoEvent>,
    cursor: Arc<AtomicU64>,
) {
    let mut stream = rsp.bytes_stream();

    {
        let (tx, rx) = oneshot::channel();
        let _ = io_tx.send(IoEvent::NewWrite(
            key.clone(),
            info.content_range
                .as_ref()
                .map(|cr| cr.full)
                .unwrap_or(info.content_length),
            cache_type,
            info.clone(),
            tx,
        ));

        match rx.await {
            Ok(true) => {}
            _ => {
                return;
            }
        }
    }

    let key = Arc::new(key);
    while let Some(bytes) = stream.next().await {
        match bytes {
            Ok(bytes) => {
                let len = bytes.len();
                let mut cursor_raw = cursor.load(std::sync::atomic::Ordering::Relaxed);
                let _ = io_tx.send(IoEvent::DoWrite(key.clone(), cursor_raw, bytes));
                cursor_raw += len as u64;
                cursor.store(cursor_raw, std::sync::atomic::Ordering::Release);
            }
            Err(e) => {
                log::error!("{:?}", e);
            }
        }
    }

    let _ = io_tx.send(IoEvent::EndWrite(key.as_ref().clone()));
}

async fn process_event_bus(
    tx: Sender<ReverseEvent>,
    mut bus_rx: Receiver<EventBus>,
    io_tx: std::sync::mpsc::Sender<IoEvent>,
) {
    let mut resource = BTreeMap::<i64, ConnectResource>::new();
    loop {
        match bus_rx.recv().await {
            Some(ev) => match ev {
                EventBus::RequestRead(key, id, cursor) => {
                    let _ = io_tx.send(IoEvent::RequestRead(key, id, cursor));
                }
                EventBus::RequestReadIoRsp(id, piece) => {
                    let _ = tx.send(ReverseEvent::RequestReadIoRsp(id, piece)).await;
                }
                EventBus::DoRealRequest(id) => {
                    if let Some(c) = resource.get(&id) {
                        if let Err(_) = c.tx.try_send(ConnectionEvent::RealRequest) {
                            resource.remove(&id);
                        }
                    }
                }
                EventBus::ReadedBuf(id, bytes, state) => {
                    if let Some(c) = resource.get(&id) {
                        if let Err(_) = c.tx.send(ConnectionEvent::ReadedBuf(bytes, state)).await {
                            resource.remove(&id);
                        }
                    }
                }
                EventBus::ReadContinue(id) => {
                    let _ = io_tx.send(IoEvent::ReadContinue(id));
                }
                EventBus::NewConnection(id, cnn_tx) => {
                    resource.insert(id, ConnectResource { tx: cnn_tx });
                }
                EventBus::EndConnection(id) => {
                    resource.remove(&id);
                    let _ = tx.send(ReverseEvent::EndConnection(id)).await;
                    let _ = io_tx.send(IoEvent::EndConnection(id));
                }
                EventBus::DoRead => {
                    let _ = io_tx.send(IoEvent::DoRead);
                }
                EventBus::NewRemoteFile(key, id, cache_type, info, rsp) => {
                    let _ = tx
                        .send(ReverseEvent::NewRemoteFile(key, id, cache_type, info, rsp))
                        .await;
                }
                EventBus::FinishFile(key, cache_type, info) => {
                    let _ = tx
                        .send(ReverseEvent::FinishFile(key, cache_type, info))
                        .await;
                }
            },
            None => {
                break;
            }
        }
    }
}

pub async fn process_cache_event(
    tx: Sender<ReverseEvent>,
    rx: Receiver<ReverseEvent>,
    db: DatabaseConnection,
    cache_dir: PathBuf,
) -> Result<()> {
    let (io_tx, io_rx) = std::sync::mpsc::channel();
    let bus_tx: Sender<EventBus> = {
        let (bus_tx, bus_rx) = tokio::sync::mpsc::channel(20);
        let tx = tx.clone();
        let io_tx = io_tx.clone();
        tokio::spawn(async move {
            process_event_bus(tx, bus_rx, io_tx).await;
        });
        bus_tx
    };
    let io_handle = {
        let bus_tx = bus_tx.clone();
        std::thread::spawn(move || {
            process_io(bus_tx, io_rx, cache_dir);
        })
    };

    let mut ctx = Box::new(ProcessCtx {
        tasks: BTreeMap::new(),
    });
    let mut rx = rx;
    let mut cnn_id: i64 = 0;

    loop {
        match rx.recv().await {
            Some(ev) => match ev {
                ReverseEvent::NewConnection(cnn, ct, rsp_tx) => {
                    let bus_tx = bus_tx.clone();
                    let (cnn_tx, cnn_rx) = tokio::sync::mpsc::channel(10);
                    let cache_info = query_cache_info(&db, &cnn.key).await;
                    cnn_id += 1;
                    let cnn_id = cnn_id;
                    let db = db.clone();
                    tokio::spawn(async move {
                        let info = {
                            match &cache_info {
                                Some(cache_info) => cache_to_remote_info(cache_info, &cnn.range),
                                None => match real_request(&ct, true, cnn.range).await {
                                    Ok((info, _)) => info,
                                    Err(e) => {
                                        log::error!("get remote file info failed: {:?}", e);
                                        return;
                                    }
                                },
                            }
                        };

                        let (rsp, stream_tx) = create_rsp(cnn_id, &cnn.range, &info);
                        log::debug!(target: "reverse", "{:?}", rsp.headers());
                        match rsp_tx.send(Ok(rsp)) {
                            Ok(_) => {
                                let _ = bus_tx.send(EventBus::NewConnection(cnn_id, cnn_tx)).await;
                                match process_connection(
                                    cnn,
                                    cnn_id,
                                    bus_tx.clone(),
                                    cnn_rx,
                                    info,
                                    stream_tx,
                                    cache_info,
                                    db,
                                    ct,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        log::debug!(
                                            target: "reverse",
                                            "stream({}) end",
                                            cnn_id
                                        );
                                    }
                                    Err(e) => {
                                        log::error!(
                                            target: "reverse",
                                            "stream({}) {:?}",
                                            cnn_id, e
                                        );
                                    }
                                }
                                let _ = bus_tx.send(EventBus::EndConnection(cnn_id)).await;
                            }
                            Err(_) => {
                                return;
                            }
                        }
                    });
                }
                ReverseEvent::NewRemoteFile(key, id, cache_type, info, rsp) => {
                    let io_tx = io_tx.clone();
                    let cursor = Arc::new(AtomicU64::new(
                        info.content_range.as_ref().map(|cr| cr.start).unwrap_or(0),
                    ));
                    let end = info
                        .content_range
                        .as_ref()
                        .map(|cr| cr.full)
                        .unwrap_or(info.content_length);
                    let handle = tokio::spawn({
                        let cursor = cursor.clone();
                        async move {
                            new_remote_file(key, cache_type, info, rsp, io_tx, cursor).await;
                        }
                    });
                    ctx.tasks.insert(
                        id,
                        RemoteFileTask {
                            cursor: cursor,
                            end,
                            handle,
                        },
                    );
                }
                ReverseEvent::RequestReadIoRsp(id, piece) => {
                    if let (None, None) = (ctx.tasks.get(&id), piece) {
                        let _ = bus_tx.send(EventBus::DoRealRequest(id)).await;
                    }
                }
                ReverseEvent::FinishFile(key, cache_type, info) => {
                    use sea_orm::{NotSet, Set};
                    if let Err(e) = sqlm::cache::Entity::insert(sqlm::cache::ActiveModel {
                        id: NotSet,
                        key: Set(key),
                        cache_type: Set(cache_type),
                        content_length: Set(info.full() as i64),
                        content_type: Set(info.content_type),
                        blob: NotSet,
                        timestamp: NotSet,
                        last_use: NotSet,
                    })
                    .on_conflict(
                        sea_query::OnConflict::columns([sqlm::cache::Column::Key])
                            .update_columns([
                                sqlm::cache::Column::CacheType,
                                sqlm::cache::Column::ContentLength,
                                sqlm::cache::Column::ContentType,
                            ])
                            .to_owned(),
                    )
                    .exec(&db)
                    .await
                    {
                        log::error!("{:?}", e);
                    }
                }
                ReverseEvent::EndConnection(id) => {
                    ctx.tasks.remove(&id);
                }
                ReverseEvent::Stop => {
                    break;
                }
            },
            None => {
                break;
            }
        }
    }

    drop(io_tx);
    let _ = io_handle.join();
    Ok(())
}

async fn process_connection(
    cnn: Connection,
    id: i64,
    bus_tx: Sender<EventBus>,
    rx: Receiver<ConnectionEvent>,
    remote_info: RemoteFileInfo,
    stream_tx: BoundedSender<StreamItem>,
    cache_info: Option<sqlm::cache::Model>,
    _db: DatabaseConnection,
    ct: Creator,
) -> Result<()> {
    let range = cnn.range;
    let mut rx = rx;
    let start = cnn.start(remote_info.full());
    let mut cursor = start;
    let mut stream_tx = stream_tx;

    match cache_info.and_then(|info| info.blob) {
        Some(bytes) => {
            log::debug!(target: "reverse", "stream({}) from db", id);
            let cursor = cursor as usize;
            let len = bytes.len();
            match bytes.as_slice().get(cursor..len) {
                Some(bytes) => {
                    let mut buf = bytes::BytesMut::new();
                    buf.put(bytes);

                    stream_tx
                        .send(Ok(hyper::body::Frame::data(buf.freeze())))
                        .await?;
                }
                _ => {
                    log::error!("out of range");
                }
            }
        }
        None => {
            let _ = bus_tx
                .send(EventBus::RequestRead(cnn.key.clone(), id, cursor))
                .await?;

            loop {
                match rx.recv().await {
                    Some(ev) => match ev {
                        ConnectionEvent::ReadedBuf(bs, state) => {
                            use futures_util::SinkExt;
                            {
                                let old = cursor;
                                cursor += bs.len() as u64;
                                log::debug!(
                                    target: "reverse",
                                    "stream({}), ({} -> {}) / {}",
                                    id,
                                    old,
                                    cursor,
                                    remote_info.full()
                                );
                            }
                            stream_tx.send(Ok(hyper::body::Frame::data(bs))).await?;

                            match state {
                                ReadState::Paused => {
                                    let _ = bus_tx.send(EventBus::ReadContinue(id)).await?;
                                }
                                ReadState::End => {
                                    if start + remote_info.content_length > cursor {
                                        log::debug!(
                                            target: "reverse",
                                            "request read: {}/{}, start: {}",
                                            cursor,
                                            remote_info.content_length,
                                            start,
                                        );
                                        let _ = bus_tx
                                            .send(EventBus::RequestRead(
                                                cnn.key.clone(),
                                                id,
                                                cursor,
                                            ))
                                            .await?;
                                    } else {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                        ConnectionEvent::RealRequest => {
                            match real_request(&ct, false, range).await {
                                Ok((info, rsp)) => {
                                    let _ = bus_tx
                                        .send(EventBus::NewRemoteFile(
                                            cnn.key.clone(),
                                            id,
                                            cnn.cache_type,
                                            info,
                                            rsp,
                                        ))
                                        .await?;
                                }
                                Err(e) => {
                                    log::error!("{:?}", e);
                                    break;
                                }
                            }
                        }
                    },
                    None => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

fn process_io(
    tx: tokio::sync::mpsc::Sender<EventBus>,
    rx: std::sync::mpsc::Receiver<IoEvent>,
    cache_dir: PathBuf,
) {
    let mut ctx = IoContext::new(&cache_dir);

    loop {
        match rx.recv() {
            Ok(ev) => match ev {
                IoEvent::EndConnection(id) => {
                    ctx.end(id);
                }
                IoEvent::EndWrite(key) => {
                    ctx.end_writer(&key);
                }
                IoEvent::RequestRead(key, id, cursor) => {
                    let p = ctx.get_piece(&key, id, cursor);
                    if p.is_none() {
                        ctx.add_waiter(id, &key, cursor);
                    }

                    let _ = tx.try_send(EventBus::RequestReadIoRsp(id, p));
                }
                IoEvent::ReadContinue(id) => {
                    ctx.set_reader_state(id, ReadState::Reading(0));
                }
                IoEvent::DoRead => {
                    ctx.do_read(
                        |id, state, bytes| {
                            let _ = tx.try_send(EventBus::ReadedBuf(id, bytes, state));
                        },
                        |id| {
                            let _ = tx.try_send(EventBus::EndConnection(id));
                        },
                    );
                }
                IoEvent::NewWrite(key, len, cache_type, remote_info, res) => {
                    let ok = ctx.new_write(&key, len, cache_type, remote_info);
                    let _ = res.send(ok);
                }
                IoEvent::DoWrite(key, offset, bytes) => {
                    if let Err(e) = ctx.do_write(
                        key.as_str(),
                        &bytes,
                        offset,
                        |key, id, start| {
                            let _ = tx.try_send(EventBus::RequestRead(key.to_string(), id, start));
                        },
                        |key, f| {
                            let _ = tx.try_send(EventBus::FinishFile(
                                key.to_string(),
                                f.cache_type,
                                f.remote_info.clone(),
                            ));
                        },
                    ) {
                        log::error!(target:"reverse", "{:?}", e);
                    }
                }
            },
            Err(_) => {
                break;
            }
        }

        for _ in 0..ctx.reading_count() {
            let _ = tx.try_send(EventBus::DoRead);
        }
    }
}
