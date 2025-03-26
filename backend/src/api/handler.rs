use super::process_ws::process_ws;
pub use super::process_ws::WsMessage;
use crate::convert::*;
use crate::global as bglobal;
use crate::msg::{self, QcmMessage, Rsp};
use crate::task::TaskManagerOper;
use crate::{
    error::ProcessError,
    event::{self, BackendContext, BackendEvent},
};
use futures_util::{SinkExt, Stream, StreamExt, TryStreamExt};
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Body, Bytes, Frame, Incoming};
use hyper::{Request, Response};
use hyper_tungstenite::HyperWebsocket;
use prost::{self, Message};
use qcm_core::provider::Context;
use qcm_core::Result;
use scopeguard::guard;
use sea_orm::{Database, DatabaseConnection};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc as async_mpsc;

use super::process_event::{process_backend_event, process_event};

pub async fn handle_request(
    mut request: Request<Incoming>,
    db: DatabaseConnection,
    oper: TaskManagerOper,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    // if ws
    if hyper_tungstenite::is_upgrade_request(&request) {
        let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;

        // spawn to handle ws connect
        tokio::spawn(async move {
            if let Err(e) = handle_ws(websocket, db, oper).await {
                eprintln!("Error in websocket connection: {e}");
            }
        });

        // return handshake
        Ok(response.map(|b| b.map_err(|_| std::io::ErrorKind::NotFound.into()).boxed()))
    } else {
        let ctx = bglobal::context(1).unwrap();
        let stream = futures_util::stream::once(async {
            Ok::<_, std::io::Error>(Frame::data(Bytes::from("Hello HTTP!")))
        });
        let body = BoxBody::new(StreamBody::new(stream));
        Ok(Response::new(body))
    }
}

async fn handle_ws(
    ws: HyperWebsocket,
    db: DatabaseConnection,
    oper: TaskManagerOper,
) -> Result<()> {
    let (mut ws_writer, ws_reader) = ws.await?.split();

    let (ws_sender, mut ws_receiver) = async_mpsc::channel::<WsMessage>(32);
    let (ev_sender, mut ev_receiver) = async_mpsc::channel::<event::Event>(32);
    let (bk_ev_sender, mut bk_ev_receiver) = async_mpsc::channel::<event::BackendEvent>(32);

    let ctx = Arc::new(BackendContext {
        provider_context: Arc::new(Context {
            db,
            ev_sender: ev_sender,
        }),
        bk_ev_sender,
        ws_sender,
        oper,
    });

    // TODO: reg context by real port
    bglobal::reg_context(1, ctx.clone());
    let _guard = guard(1, |p| {
        bglobal::unreg_context(p);
    });

    // ws sender queue
    tokio::spawn({
        async move {
            while let Some(msg) = ws_receiver.recv().await {
                if ws_writer.send(msg.into()).await.is_err() {
                    break;
                }
            }
            log::info!("Channel recv end");
        }
    });

    // event queue
    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            while let Some(ev) = ev_receiver.recv().await {
                match process_event(ev, ctx.clone()).await {
                    Ok(true) => break,
                    Err(err) => log::error!("{}", err),
                    _ => (),
                }
            }
            log::info!("Event channel recv end");
        }
    });

    // backend event queue
    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            while let Some(ev) = bk_ev_receiver.recv().await {
                match process_backend_event(ev, ctx.clone()).await {
                    Ok(true) => break,
                    Err(err) => log::error!("{}", err),
                    _ => (),
                }
            }
            log::info!("Backend event channel recv end");
        }
    });

    let _ = ctx.bk_ev_sender.try_send(BackendEvent::Frist);

    // receive from ws
    // let mut read = ws_reader.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()));
    let mut reader = ws_reader;
    while let Ok(Some(message)) = reader.next().await.transpose() {
        tokio::spawn({
            let ctx = ctx.clone();
            async move {
                if let Err(e) = handle_ws_message(message, ctx).await {
                    log::warn!("Error processing message: {}", e);
                }
            }
        });
    }

    // end event process
    ctx.provider_context
        .ev_sender
        .send(event::Event::End)
        .await
        .unwrap();
    ctx.bk_ev_sender
        .send(event::BackendEvent::End)
        .await
        .unwrap();
    return Ok(());
}

pub async fn handle_ws_message(msg: WsMessage, ctx: Arc<BackendContext>) -> Result<()> {
    use msg::qcm_message::Payload;
    let mut id: Option<i32> = None;

    match process_ws(&ctx, &ctx.ws_sender, &msg, &mut id).await {
        Ok(msg_rsp) => {
            log::warn!("send {}", msg_rsp.r#type);
            let mut buf = Vec::new();
            msg_rsp.encode(&mut buf)?;
            ctx.ws_sender.send(WsMessage::Binary(buf.into())).await?;
        }
        Err(ProcessError::None) => {}
        Err(err) => {
            if let Some(id) = id {
                let rsp: Rsp = err.qcm_into();
                let msg = QcmMessage {
                    id,
                    r#type: msg::MessageType::Rsp.into(),
                    payload: Some(Payload::Rsp(rsp)),
                };
                log::warn!("send {}", msg.r#type);
                let mut buf = Vec::new();
                msg.encode(&mut buf)?;
                ctx.ws_sender.send(WsMessage::Binary(buf.into())).await?;
            } else {
                log::error!("{}", err);
            }
        }
    }
    Ok(())
}
