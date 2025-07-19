use super::connection::{ConnectionEvent, RemoteFileInfo};
use crate::error::ProcessError;
use crate::http::{body_type::StreamItem, range::HttpRange};
use bytes::Bytes;
use qcm_core::model as sqlm;
use qcm_core::model::type_enum::CacheType;
use sea_orm::DatabaseConnection;

use super::bus::EventBus;
use super::connection::{
    cache_to_remote_info, create_rsp, real_request, Connection, Creator, ResponceOneshot,
};
use super::io::ReadState;
use strum_macros::Display;

use futures::channel::mpsc::Sender as BoundedSender;

#[derive(Debug, Default, Display)]
enum ConnectionState {
    #[default]
    Init,
    QueryingFileInfo,
    SendResponse,
    WaitingForPiece,
    QueryRemoteFile,
    Serving(Bytes, bool /*last */),
    ServingFromDB(Bytes),
    BusClosed,
    Finished,
    Error(ProcessError),
}

// impl std::fmt::Display for ConnectionState {
// }

pub struct ConnectionHandler {
    state: ConnectionState,
    id: i64,
    key: String,
    range: Option<HttpRange>,
    cache_type: CacheType,

    bus_tx: tokio::sync::mpsc::Sender<EventBus>,
    rx: tokio::sync::mpsc::Receiver<ConnectionEvent>,
    rsp_tx: Option<ResponceOneshot>,
    stream_tx: Option<BoundedSender<StreamItem>>,

    db: DatabaseConnection,

    ct: Creator,

    //
    cache_info: Option<sqlm::cache::Model>,
    file_info: RemoteFileInfo,
    cursor: u64,
    start: u64,
}

impl ConnectionHandler {
    pub async fn process(
        cnn: Connection,
        id: i64,
        ct: Creator,
        db: DatabaseConnection,
        bus_tx: tokio::sync::mpsc::Sender<EventBus>,
        rsp_tx: ResponceOneshot,
    ) {
        let (cnn_tx, cnn_rx) = tokio::sync::mpsc::channel(10);

        let mut handler = ConnectionHandler {
            state: ConnectionState::Init,
            id: id,
            key: cnn.key,
            range: cnn.range,
            cache_type: cnn.cache_type,
            bus_tx: bus_tx.clone(),
            rx: cnn_rx,
            rsp_tx: Some(rsp_tx),
            stream_tx: None,
            db: db,
            ct: ct,
            cache_info: None,
            file_info: RemoteFileInfo::default(),
            cursor: 0,
            start: 0,
        };

        if handler.bus_send(EventBus::NewConnection(id, cnn_tx)).await {
            while handler.poll_once().await {}
        }

        handler.bus_send(EventBus::EndConnection(id)).await;
    }

    async fn servring_bytes(&mut self, bytes: Bytes) -> bool {
        use futures_util::SinkExt;
        match &mut self.stream_tx {
            Some(stream_tx) => {
                if let Err(_) = stream_tx.send(Ok(hyper::body::Frame::data(bytes))).await {
                    log::warn!(target: "reverse", "connection already closed");
                    return false;
                }
            }
            None => {
                log::warn!(target: "reverse", "connection not opened");
                return false;
            }
        }
        return true;
    }

    async fn bus_send(&mut self, ev: EventBus) -> bool {
        if let Err(_) = self.bus_tx.send(ev).await {
            self.state = ConnectionState::BusClosed;
            return false;
        }
        return true;
    }

    async fn request_read(&mut self) -> bool {
        log::debug!(
            target: "reverse",
            "cnn {} request read: {}/{}, start: {}",
            self.id,
            self.cursor,
            self.file_info.content_length,
            self.start,
        );
        let ev = EventBus::RequestRead(
            self.key.clone(),
            self.id,
            self.cursor,
            self.cache_info.is_some(),
        );
        return self.bus_send(ev).await;
    }

    async fn poll_once(&mut self) -> bool {
        log::debug!(target: "reverse", "cnn {} state: {}", self.id, self.state);
        match &self.state {
            ConnectionState::Init => {
                self.cursor = 0;
                self.start = 0;
                self.state = ConnectionState::QueryingFileInfo;
            }
            ConnectionState::QueryingFileInfo => {
                self.cache_info = sqlm::cache::query_by_key(&self.db, &self.key).await;
                match &self.cache_info {
                    Some(cache_info) => {
                        self.file_info = cache_to_remote_info(cache_info, &self.range);
                    }
                    None => match real_request(&self.ct, true, self.range.clone()).await {
                        Ok((info, _)) => {
                            self.file_info = info;
                        }
                        Err(e) => {
                            log::error!("get remote file info failed: {:?}", e);
                            self.state = ConnectionState::Error(e);
                            return true;
                        }
                    },
                };

                // update start from range
                self.start = match &self.range {
                    Some(r) => r.start(self.file_info.full()),
                    None => 0,
                };
                self.cursor = self.start;
                self.state = ConnectionState::SendResponse;
            }
            ConnectionState::SendResponse => {
                let (rsp, stream_tx) = create_rsp(self.id, &self.range, &self.file_info);
                log::debug!(target: "reverse", "{:?}", rsp.headers());
                match self.rsp_tx.take() {
                    Some(rsp_tx) => match rsp_tx.send(Ok(rsp)) {
                        Ok(_) => match self.cache_info.as_ref().and_then(|c| c.blob.clone()) {
                            Some(b) => {
                                self.stream_tx = Some(stream_tx);
                                self.state =
                                    ConnectionState::ServingFromDB(Bytes::copy_from_slice(&b));
                            }
                            None => {
                                self.stream_tx = Some(stream_tx);
                                if self.request_read().await {
                                    self.state = ConnectionState::WaitingForPiece;
                                }
                            }
                        },
                        Err(_) => {
                            self.state = ConnectionState::Finished;
                        }
                    },
                    None => {
                        self.state = ConnectionState::Finished;
                    }
                }
            }
            ConnectionState::ServingFromDB(bytes) => {
                log::debug!(target: "reverse", "cnn {} stream from db", self.id);
                let cursor = self.start as usize;
                let len = bytes.len();

                if cursor < len {
                    let bytes = bytes.slice(cursor..len);
                    self.servring_bytes(bytes).await;
                } else {
                    log::error!("out of range");
                }
                self.state = ConnectionState::Finished;
            }
            ConnectionState::Serving(bytes, last) => {
                let last = *last;

                if !self.servring_bytes(bytes.clone()).await {
                    self.state = ConnectionState::Finished;
                } else if last {
                    self.state = ConnectionState::Finished;
                } else {
                    self.state = ConnectionState::WaitingForPiece;
                }
            }
            ConnectionState::WaitingForPiece => match self.rx.recv().await {
                Some(ev) => match ev {
                    ConnectionEvent::ReadedBuf(bs, state) => {
                        {
                            let old = self.cursor;
                            self.cursor += bs.len() as u64;
                            let cursor = self.cursor;
                            let full = self.file_info.full();
                            log::debug!(
                                target: "reverse",
                                "cnn {} streaming: ({} -> {}) / {}",
                                self.id,
                                old,
                                self.cursor,
                                self.file_info.full()
                            );
                            if cursor > full {
                                log::error!(target: "reverse", "cnn {} overflow", self.id);
                            }
                        }

                        match state {
                            ReadState::Paused => {
                                if !self.bus_send(EventBus::ReadContinue(self.id)).await {
                                    return true;
                                }
                            }
                            ReadState::End => {
                                if self.start + self.file_info.content_length > self.cursor {
                                    if !self.request_read().await {
                                        return true;
                                    }
                                } else {
                                    // last serving
                                    self.state = ConnectionState::Serving(bs, true);
                                    return true;
                                }
                            }
                            _ => {}
                        }

                        self.state = ConnectionState::Serving(bs, false);
                    }
                    ConnectionEvent::NoCache => {
                        self.state = ConnectionState::QueryRemoteFile;
                    }
                },
                None => {
                    self.state = ConnectionState::Finished;
                }
            },
            ConnectionState::QueryRemoteFile => {
                for attempt in 1..=3 {
                    match real_request(&self.ct, false, self.range.clone()).await {
                        Ok((info, rsp)) => {
                            let ev = EventBus::NewRemoteFile(
                                self.key.clone(),
                                self.id,
                                self.cache_type,
                                info,
                                rsp,
                            );
                            if !self.bus_send(ev).await {
                                self.state = ConnectionState::Finished;
                            } else {
                                self.state = ConnectionState::WaitingForPiece;
                            }
                            break;
                        }
                        Err(e) => {
                            log::error!("{:?}", e);
                            self.state = ConnectionState::Error(e);
                            // wait
                            if attempt < 3 {
                                // 1s -> 2s -> 4s
                                let backoff = std::time::Duration::from_secs(2u64.pow(attempt - 1));
                                tokio::time::sleep(backoff).await;
                            }
                        }
                    }
                }
            }
            ConnectionState::BusClosed => {
                return false;
            }
            ConnectionState::Error(_) => {
                return false;
            }
            ConnectionState::Finished => {
                return false;
            }
        };
        return true;
    }
}
