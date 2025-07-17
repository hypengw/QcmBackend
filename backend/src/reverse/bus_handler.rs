use std::collections::BTreeMap;
use tokio::sync::mpsc::{Receiver, Sender};

use super::bus::EventBus;
use super::connection::ConnectionEvent;
use super::io::IoEvent;
use super::reverse::ReverseEvent;
struct ConnectResource {
    pub tx: Sender<ConnectionEvent>,
}

pub async fn process_event_bus(
    tx: Sender<ReverseEvent>,
    mut bus_rx: Receiver<EventBus>,
    io_tx: std::sync::mpsc::Sender<IoEvent>,
) {
    let mut resource = BTreeMap::<i64, ConnectResource>::new();
    loop {
        match bus_rx.recv().await {
            Some(ev) => match ev {
                EventBus::RequestRead(key, id, cursor, has_cache) => {
                    let _ = io_tx.send(IoEvent::RequestRead(key, id, cursor, has_cache));
                }
                EventBus::NoCache(id) => {
                    let _ = tx.send(ReverseEvent::HasRemoteFile(id)).await;
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
                    log::debug!(target: "reverse", "new connection {}, active: {}", id, resource.len());
                }
                EventBus::EndConnection(id) => {
                    resource.remove(&id);
                    let _ = tx.send(ReverseEvent::EndConnection(id)).await;
                    let _ = io_tx.send(IoEvent::EndConnection(id));
                    log::debug!(target: "reverse", "end connection {}, active: {}", id, resource.len());
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
                EventBus::DbFinishFile(key) => {
                    let _ = io_tx.send(IoEvent::EndWrite(key));
                }
                EventBus::EndRemoteFile(id) => {
                    let _ = tx.send(ReverseEvent::EndRemoteFile(id)).await;
                }
                EventBus::NoRemoteFile(id) => {
                    if let Some(c) = resource.get(&id) {
                        if let Err(_) = c.tx.try_send(ConnectionEvent::NoCache) {
                            resource.remove(&id);
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
