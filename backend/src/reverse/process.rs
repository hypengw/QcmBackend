use hyper_util::client::legacy::connect::Connected;
use qcm_core::Result;

use super::body_type::ResponseBody;
use super::connection::Connection;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

pub enum ReverseEvent {
    NewConnection(Connection, oneshot::Sender<Result<hyper::Response<ResponseBody>>>),
    Stop,
}

enum ConnectionEvent {}

struct ProcessCtx {}

pub async fn process_cache_event(rx: &mut Receiver<ReverseEvent>) -> Result<()> {
    let ctx = Box::new(ProcessCtx {});
    match rx.recv().await {
        Some(ev) => {
            match ev {
                ReverseEvent::NewConnection(c) => {
                    tokio::spawn(async move {
                        // if let Err(e) = process_connection(c, &mut rx).await {
                        //     log::error!("Error processing connection: {}", e);
                        // }
                    });
                }
                ReverseEvent::Stop => {}
            }
        }
        None => {}
    }
    Ok(())
}

async fn process_connection(c: Connection, rx: &mut Receiver<ConnectionEvent>) -> Result<()> {
    match rx.recv().await {
        Some(_) => {}
        None => {}
    }
    Ok(())
}
