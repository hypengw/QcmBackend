use super::process_http::process_http_get;
use super::process_ws::process_ws;
pub use super::process_ws::WsMessage;
use crate::api::process_http::process_http_post;
use crate::convert::*;
use crate::global as bglobal;
use crate::http::body_type::ResponseBody;
use crate::msg::{self, QcmMessage, Rsp};
use crate::reverse::ReverseEvent;
use crate::task::TaskManagerOper;
use crate::{
    error::ProcessError,
    event::{self, BackendContext, BackendEvent},
};
use futures_util::{SinkExt, Stream, StreamExt, TryStreamExt};
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Body, Bytes, Frame, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper_tungstenite::HyperWebsocket;
use prost::{self, Message};
use qcm_core::provider::Context;
use qcm_core::Result;
use scopeguard::guard;
use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;
use tokio::sync::mpsc as async_mpsc;

use super::process_event::{process_backend_event, process_event, ProcessContext};

pub async fn handle_request(
    mut request: Request<Incoming>,
    db: DatabaseConnection,
    cache_db: DatabaseConnection,
    oper: TaskManagerOper,
    reverse_ev: tokio::sync::mpsc::Sender<ReverseEvent>,
) -> Result<Response<ResponseBody>> {
    // if ws
    if hyper_tungstenite::is_upgrade_request(&request) {
        let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;

        // spawn to handle ws connect
        tokio::spawn(async move {
            if let Err(e) = handle_ws(websocket, db, cache_db, oper, reverse_ev).await {
                log::error!("Error in websocket connection: {e}");
            }
        });

        // return upgrade rsp
        Ok(response.map(|b| ResponseBody::Boxed(b.map_err(|e| e.into()).boxed())))
    } else {
        // TODO: support multiple backend
        let ctx = bglobal::context(1).unwrap();
        log::debug!(target: "http", "{}", request.uri());

        use hyper::Method;
        let res = match *request.method() {
            Method::GET => process_http_get(&ctx, request).await,
            Method::POST => process_http_post(&ctx, request).await,
            _ => {
                let rsp = Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(ResponseBody::Empty)
                    .unwrap();
                Ok(rsp)
            }
        };

        match res {
            Ok(rsp) => Ok(rsp),
            Err(
                ProcessError::NotFound
                | ProcessError::NoSuchAlbum(_)
                | ProcessError::NoSuchArtist(_)
                | ProcessError::NoSuchSong(_)
                | ProcessError::NoSuchMix(_)
                | ProcessError::NoSuchItemType(_),
            ) => {
                let rsp = Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(ResponseBody::Empty)
                    .unwrap();
                Ok(rsp)
            }
            Err(err) => Err(err.into()),
        }
    }
}

async fn handle_ws(
    ws: HyperWebsocket,
    db: DatabaseConnection,
    cache_db: DatabaseConnection,
    oper: TaskManagerOper,
    reverse_ev: tokio::sync::mpsc::Sender<ReverseEvent>,
) -> Result<()> {
    let (mut ws_writer, ws_reader) = ws.await?.split();

    let (ws_sender, mut ws_receiver) = async_mpsc::channel::<WsMessage>(32);
    let (ev_sender, mut ev_receiver) = async_mpsc::channel::<event::Event>(1024);
    let (bk_ev_sender, mut bk_ev_receiver) = async_mpsc::channel::<event::BackendEvent>(1024);

    let ctx = Arc::new(BackendContext {
        provider_context: Arc::new(Context {
            db,
            cache_db,
            ev_sender: ev_sender,
        }),
        backend_ev: bk_ev_sender,
        ws_sender,
        oper,
        reverse_ev,
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
            let mut process_ctx = ProcessContext::new();
            while let Some(ev) = bk_ev_receiver.recv().await {
                match process_backend_event(ev, ctx.clone(), &mut process_ctx).await {
                    Ok(true) => break,
                    Err(err) => log::error!("{}", err),
                    _ => (),
                }
            }
            log::info!("Backend event channel recv end");
        }
    });

    let _ = ctx.backend_ev.try_send(BackendEvent::Frist);

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

    log::info!("WebSocket connection closed");

    // end event process
    ctx.provider_context
        .ev_sender
        .send(event::Event::End)
        .await
        .unwrap();
    ctx.backend_ev.send(event::BackendEvent::End).await.unwrap();

    // Only support one client, close self
    bglobal::shutdown();
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
