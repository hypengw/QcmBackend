use futures::StreamExt;
use qcm_core::model as sqlm;
use qcm_core::model::type_enum::CacheType;
use qcm_core::Result;
use sea_orm::sea_query;
use sea_orm::{DatabaseConnection, EntityTrait};

use super::connection::RemoteFileInfo;
use reqwest::Response;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

use super::bus::EventBus;
use super::io::IoEvent;
use super::io_handler::process_io;
use super::reverse::ReverseEvent;
use crate::task::TaskManagerOper;

struct RemoteFileTask {
    pub key: String,
    pub cursor: Arc<AtomicU64>,
    pub end: u64,
    pub task_id: i64,
}
pub struct ReverseHandler {
    cnn_id: i64,
    rx: Receiver<ReverseEvent>,
    db: DatabaseConnection,
    bus_tx: Sender<EventBus>,
    io_tx: std::sync::mpsc::Sender<IoEvent>,
    task_oper: TaskManagerOper,
    tasks: BTreeMap<i64, RemoteFileTask>,
}

impl ReverseHandler {
    pub async fn process(
        tx: Sender<ReverseEvent>,
        rx: Receiver<ReverseEvent>,
        db: DatabaseConnection,
        task_oper: TaskManagerOper,
        cache_dir: PathBuf,
    ) -> Result<()> {
        let (io_tx, io_rx) = std::sync::mpsc::channel();
        let bus_tx: Sender<EventBus> = {
            let (bus_tx, bus_rx) = tokio::sync::mpsc::channel(20);
            let tx = tx.clone();
            let io_tx = io_tx.clone();
            tokio::spawn(async move {
                super::bus_handler::process_event_bus(tx, bus_rx, io_tx).await;
            });
            bus_tx
        };
        let io_handle = {
            let bus_tx = bus_tx.clone();
            std::thread::spawn(move || {
                process_io(bus_tx, io_rx, cache_dir);
            })
        };

        let mut ctx = ReverseHandler {
            cnn_id: 0,
            rx,
            db,
            bus_tx,
            io_tx,
            task_oper,
            tasks: BTreeMap::new(),
        };
        while ctx.poll_once().await {}

        drop(ctx);
        let _ = io_handle.join();
        Ok(())
    }

    fn remove_task(&mut self, id: i64, cancel: bool) {
        if cancel {
            if let Some(t) = self.tasks.get(&id) {
                self.task_oper.canel(t.task_id);
            }
        }
        self.tasks.remove(&id);
    }

    fn cancel_task_by_key(&mut self, key: &str) {
        let mut to_rm: Vec<i64> = Vec::new();
        for (k, v) in self.tasks.iter() {
            if v.key == key {
                to_rm.push(*k);
            }
        }
        for k in to_rm {
            self.remove_task(k, true);
        }
    }

    async fn poll_once(&mut self) -> bool {
        match self.rx.recv().await {
            Some(ev) => match ev {
                ReverseEvent::NewConnection(cnn, ct, rsp_tx) => {
                    self.cnn_id = (self.cnn_id + 1) % 65536;
                    let cnn_id = self.cnn_id;
                    let db = self.db.clone();
                    let bus_tx = self.bus_tx.clone();
                    tokio::spawn(async move {
                        use super::connection_handler::ConnectionHandler;
                        ConnectionHandler::process(cnn, cnn_id, ct, db, bus_tx, rsp_tx).await;
                    });
                }
                ReverseEvent::NewRemoteFile(key, id, cache_type, info, rsp) => {
                    let io_tx = self.io_tx.clone();
                    let cursor = Arc::new(AtomicU64::new(
                        info.content_range.as_ref().map(|cr| cr.start).unwrap_or(0),
                    ));
                    let end = info
                        .content_range
                        .as_ref()
                        .map(|cr| cr.full)
                        .unwrap_or(info.content_length);

                    let task_id = self.task_oper.spawn({
                        let cursor = cursor.clone();
                        let key = key.clone();
                        let bus_tx = self.bus_tx.clone();
                        let id = id;
                        async move |_| {
                            new_remote_file(key, cache_type, info, rsp, io_tx, cursor).await;
                            let _ = bus_tx.send(EventBus::EndRemoteFile(id)).await;
                        }
                    });
                    self.tasks.insert(
                        id,
                        RemoteFileTask {
                            key,
                            cursor: cursor,
                            end,
                            task_id,
                        },
                    );

                    log::debug!(target: "reverse", "newRemoteFile {}, active: {}", id, self.tasks.len());
                }
                ReverseEvent::EndRemoteFile(id) => {
                    self.remove_task(id, false);
                }
                ReverseEvent::FinishFile(key, cache_type, info) => {
                    use sea_orm::{NotSet, Set};
                    match sqlm::cache::Entity::insert(sqlm::cache::ActiveModel {
                        id: NotSet,
                        key: Set(key.clone()),
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
                    .exec(&self.db)
                    .await
                    {
                        Err(e) => {
                            log::error!("{:?}", e);
                        }
                        Ok(_) => {}
                    }

                    self.cancel_task_by_key(&key);
                    let _ = self.bus_tx.send(EventBus::DbFinishFile(key)).await;
                }
                ReverseEvent::HasRemoteFile(id) => {
                    if !self.tasks.contains_key(&id) {
                        let _ = self.bus_tx.send(EventBus::NoRemoteFile(id)).await;
                    }
                }
                ReverseEvent::EndConnection(id) => {
                    self.remove_task(id, false);
                }
                ReverseEvent::Stop => {
                    return false;
                }
            },
            None => {
                return false;
            }
        }
        return true;
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
                break;
            }
        }
    }

    let _ = io_tx.send(IoEvent::EndWrite(key.as_ref().clone()));
}
