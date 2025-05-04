use bytes::{Buf, Bytes};
use futures::channel::mpsc::UnboundedSender;
use futures::SinkExt;
use http_body_util::StreamBody;
use hyper_util::client::legacy::connect::Connected;
use qcm_core::model as sqlm;
use qcm_core::Result;
use sea_orm::QuerySelect;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter};

use super::body_type::{ResponseBody, StreamItem};
use super::connection::{default_range, parse_content_range, Connection, ContentRange, Range};
use super::piece;
use crate::error::{HttpError, ProcessError};
use crate::reverse::connection::range_in_full;
use reqwest::Response;
use std::collections::BTreeMap;
use std::future::Future;
use std::io::{Seek, Write};
use std::ops::Deref;
use std::path::PathBuf;
use std::pin::Pin;
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
    ReadedBuf(i64, u64, Bytes),
    ReadFinish(i64, u64, bool),
    RealRequest(i64),
    Write(i64, u64, Bytes),
    EndConnection(i64),
}

pub enum ReverseEvent {
    NewConnection(
        Connection,
        Creator,
        oneshot::Sender<Result<hyper::Response<ResponseBody>, hyper::Error>>,
    ),
    AddCommon(String, ConnectCommonResource),
    EndConnection(String),
    Stop,
}

pub fn wrap_creator<Fut>(ct: impl Fn(bool, Option<Range>) -> Fut + Send + Sync + 'static) -> Creator
where
    Fut: Future<Output = Result<Response, ProcessError>> + Send + 'static,
{
    Box::new(move |head: bool, r: Option<Range>| Box::pin(ct(head, r)))
}

enum ConnectionEvent {
    ReadedBuf(Bytes),
    ReadFinish(u64, bool),
    RealRequest,
}

enum IoEvent {
    RequestRead(String, i64),
    Read(String, i64),
    Write(String, u64, Bytes, Option<(PathBuf, u64)>),
}

struct ConnectCommonResource {
    info: RemoteFileInfo,
}

struct ConnectResource {
    pub tx: Sender<ConnectionEvent>,
}

struct ProcessCtx {
    pub common: BTreeMap<String, ConnectCommonResource>,
}

#[derive(Clone, Debug, PartialEq)]
struct RemoteFileInfo {
    content_type: String,
    content_length: u64,
    accept_ranges: bool,
    content_range: Option<ContentRange>,
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
            let accept_ranges = headers.contains_key(reqwest::header::ACCEPT_RANGES);
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

pub async fn process_cache_event(
    tx: Sender<ReverseEvent>,
    rx: Receiver<ReverseEvent>,
    db: DatabaseConnection,
) -> Result<()> {
    let mut ctx = Box::new(ProcessCtx {
        common: BTreeMap::new(),
    });
    let mut rx = rx;

    let (io_tx, io_handle) = {
        let tx = tx.clone();
        let (io_tx, io_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            process_io(tx, io_rx);
        });
        (io_tx, handle)
    };

    let bus_tx: Sender<EventBus> = {
        let (tx, mut rx) = tokio::sync::mpsc::channel(20);
        tokio::spawn(async move {
            let resource = BTreeMap::<i64, ConnectResource>::new();
            loop {
                match rx.recv().await {
                    _ => {}
                }
            }
        });
        tx
    };

    let mut cnn_id: i64 = 0;

    use sea_orm::ColumnTrait;

    loop {
        match rx.recv().await {
            Some(ev) => match ev {
                ReverseEvent::NewConnection(cnn, ct, rsp_tx) => {
                    let ev_tx = tx.clone();
                    let bus_tx = bus_tx.clone();
                    let (cnn_tx, cnn_rx) = tokio::sync::mpsc::channel(10);
                    let cache_info = sqlm::cache::Entity::find()
                        .select_only()
                        .filter(sqlm::cache::Column::Key.eq(cnn.key.clone()))
                        .into_partial_model::<sqlm::cache::Info>()
                        .one(&db)
                        .await?;

                    cnn_id += 1;
                    let cnn_id = cnn_id;
                    let db = db.clone();
                    tokio::spawn(async move {
                        let info = {
                            match &cache_info {
                                Some(cache_info) => RemoteFileInfo {
                                    content_length: cache_info.content_length,
                                    content_type: cache_info.content_type.clone(),
                                    accept_ranges: true,
                                    content_range: cnn.range.clone().and_then(|r| {
                                        ContentRange::from_range(r, cache_info.content_length)
                                    }),
                                },
                                None => match real_request(&ct, true, cnn.range).await {
                                    Ok((info, _)) => {
                                        let _ = ev_tx
                                            .send(ReverseEvent::AddCommon(
                                                cnn.key.clone(),
                                                ConnectCommonResource { info: info.clone() },
                                            ))
                                            .await;

                                        info
                                    }
                                    Err(_) => {
                                        log::error!("get remote file info failed");
                                        return;
                                    }
                                },
                            }
                        };

                        let (stream_tx, stream_rx) = futures::channel::mpsc::unbounded();
                        let rsp = {
                            match (cnn.range, &info.content_range) {
                                (Some(r), Some(cr)) => {
                                    if range_in_full(&r, cr.full) {
                                        hyper::Response::builder()
                                            .status(hyper::StatusCode::PARTIAL_CONTENT)
                                            .header(hyper::header::CONTENT_TYPE, &info.content_type)
                                            .header(
                                                hyper::header::CONTENT_LENGTH,
                                                info.content_length,
                                            )
                                            .header(hyper::header::ACCEPT_RANGES, "bytes")
                                            .header(hyper::header::CONTENT_RANGE, cr.to_string())
                                            .body(ResponseBody::UnboundedStreamed(StreamBody::new(
                                                stream_rx,
                                            )))
                                            .unwrap()
                                    } else {
                                        hyper::Response::builder()
                                            .status(hyper::StatusCode::RANGE_NOT_SATISFIABLE)
                                            .header(hyper::header::CONTENT_TYPE, &info.content_type)
                                            .header(hyper::header::ACCEPT_RANGES, "bytes")
                                            .header(
                                                hyper::header::CONTENT_RANGE,
                                                format!("bytes */{}", cr.full),
                                            )
                                            .body(ResponseBody::Empty)
                                            .unwrap()
                                    }
                                }
                                (_, _) => {
                                    let mut b = hyper::Response::builder()
                                        .status(hyper::StatusCode::OK)
                                        .header(hyper::header::CONTENT_TYPE, &info.content_type)
                                        .header(hyper::header::CONTENT_LENGTH, info.content_length);

                                    if info.accept_ranges {
                                        b = b.header(hyper::header::ACCEPT_RANGES, "bytes");
                                    }

                                    b.body(ResponseBody::UnboundedStreamed(StreamBody::new(
                                        stream_rx,
                                    )))
                                    .unwrap()
                                }
                            }
                        };
                        match rsp_tx.send(Ok(rsp)) {
                            Ok(_) => {
                                let _ = process_connection(
                                    cnn, cnn_id, bus_tx, cnn_rx, info, stream_tx, cache_info, db,
                                    ct,
                                )
                                .await;
                            }
                            Err(_) => {
                                return;
                            }
                        }
                    });
                }
                ReverseEvent::AddCommon(key, common) => {
                    ctx.common.insert(key, common);
                }
                ReverseEvent::EndConnection(key) => {}
                ReverseEvent::Stop => {}
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
    stream_tx: UnboundedSender<StreamItem>,
    cache_info: Option<sqlm::cache::Info>,
    db: DatabaseConnection,
    ct: Creator,
) -> Result<()> {
    let range = cnn.range;
    let mut rx = rx;
    let mut cursor: u64 = match range.map(|r| r.start) {
        Some(http_range_header::StartPosition::Index(pos)) => pos,
        Some(http_range_header::StartPosition::FromLast(pos)) => match remote_info.content_range {
            Some(cr) => {
                if cr.full < pos {
                    log::error!("");
                    return Ok(());
                } else {
                    cr.full - (pos + 1)
                }
            }
            None => {
                log::error!("unknown remote file size");
                return Ok(());
            }
        },
        None => 0,
    };
    let mut stream_tx = stream_tx;
    let mut retry = 3;

    match cache_info {
        Some(cache_info) => {
            while cursor != cache_info.content_length {
                let size = std::cmp::min(512, cache_info.content_length - cursor);
                match sqlm::cache::blob_chunk(&db, Some(cache_info.id), None, cursor, 512).await {
                    Ok(bytes) => stream_tx.send(Ok(hyper::body::Frame::data(bytes))).await?,
                    Err(e) => {
                        log::error!("{:?}", e);
                        return Ok(());
                    }
                };
                cursor += size;
            }
        }
        None => {
            let _ = bus_tx
                .send(EventBus::RequestRead(cnn.key.clone(), id, cursor))
                .await?;

            loop {
                match rx.recv().await {
                    Some(ev) => match ev {
                        ConnectionEvent::ReadedBuf(bs) => {
                            use futures_util::SinkExt;
                            stream_tx.send(Ok(hyper::body::Frame::data(bs))).await?;
                        }
                        ConnectionEvent::ReadFinish(pos, finished) => {
                            cursor = pos;
                            if finished {
                                break;
                            }

                            let _ = bus_tx.send(EventBus::RequestRead(cnn.key.clone(), id, cursor));
                        }
                        ConnectionEvent::RealRequest => {
                            match real_request(&ct, false, range).await {
                                Ok((_info, rsp)) => {}
                                Err(e) => {
                                    log::error!("{:?}", e);
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

struct DownloadFile {
    meta: piece::FileMeta,
    file: std::fs::File,
}

fn process_io(tx: tokio::sync::mpsc::Sender<ReverseEvent>, rx: std::sync::mpsc::Receiver<IoEvent>) {
    let mut readers = BTreeMap::<i64, std::fs::File>::new();
    let mut writers = BTreeMap::<String, DownloadFile>::new();
    loop {
        match rx.recv() {
            Ok(ev) => match ev {
                IoEvent::RequestRead(key, id) => {}
                IoEvent::Read(key, id) => {
                    match readers.get_mut(&id) {
                        Some(file) => {}
                        None => {}
                    }
                    // if let Some(c) = contents.get(&key) {
                    //     // match c {}
                    // }
                }
                IoEvent::Write(key, offset, bytes, path_and_len) => {
                    match (writers.contains_key(&key), path_and_len) {
                        (false, Some((path, len))) => {
                            match std::fs::File::create(&path)
                                .and_then(|f| f.set_len(len).map(|_| f))
                            {
                                Ok(file) => {
                                    writers.insert(
                                        key.clone(),
                                        DownloadFile {
                                            meta: piece::FileMeta {
                                                path,
                                                len,
                                                pieces: BTreeMap::new(),
                                            },
                                            file,
                                        },
                                    );
                                }
                                Err(e) => {
                                    log::error!("{:?}", e);
                                    return;
                                }
                            }
                        }
                        (false, None) => {
                            log::error!("no writer and no path");
                            return;
                        }
                        (_, _) => {}
                    }

                    let f = writers.get_mut(&key).unwrap();
                    let seeked = f.file.seek(std::io::SeekFrom::Start(offset));
                    if seeked.map(|s| s != offset).unwrap_or(true) {
                        let path = f.meta.path.clone();
                        log::error!("seek file failed: {}", &path.to_str().unwrap_or_default());
                        writers.remove(&key);
                        let _ = std::fs::remove_file(path.clone());
                        return;
                    }
                    let writed = f.file.write(&bytes);
                    if writed.map(|w| w != bytes.len()).unwrap_or(true) {
                        let path = f.meta.path.clone();
                        log::error!("write file failed: {}", &path.to_str().unwrap_or_default());
                        writers.remove(&key);
                        let _ = std::fs::remove_file(path.clone());
                        return;
                    }
                }
            },
            Err(_) => {
                break;
            }
        }
    }
}
