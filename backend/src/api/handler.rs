use prost::{self, Message};
use qcm_core::{global, Result};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use tokio::sync::mpsc::Sender;

use crate::convert::QcmInto;
use crate::error::ProcessError;
use crate::event::BackendContext;
use crate::msg::{self, GetProviderMetasRsp, MessageType, QcmMessage, Rsp, TestRsp};

type TX = Sender<WsMessage>;

fn wrap(in_msg: &QcmMessage, playload: msg::qcm_message::Payload) -> QcmMessage {
    let message = QcmMessage {
        r#type: in_msg.r#type() as i32,
        id: in_msg.id.clone(),
        payload: Some(playload),
    };
    return message;
}

async fn process_message(
    ctx: &Arc<BackendContext>,
    tx: &TX,
    msg: &WsMessage,
    in_id: &mut Option<i32>,
) -> Result<QcmMessage, ProcessError> {
    use msg::qcm_message::Payload;
    match msg {
        WsMessage::Binary(data) => {
            let message = QcmMessage::decode(&data[..])?;
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
                    return Ok(wrap(&message, Payload::GetProviderMetasRsp(response)));
                }
                MessageType::AddProviderReq => {
                    log::warn!("add provider");
                    if let Some(Payload::AddProviderReq(req)) = payload {
                        if let Some(auth_info) = &req.auth_info {
                            if let Some(meta) = qcm_core::global::provider_meta(&req.type_name) {
                                let provider =
                                    (meta.creator)(None, &req.name, &global::device_id());
                                provider
                                    .login(&ctx.provider_context, &auth_info.clone().qcm_into())
                                    .await?;
                                return Ok(wrap(&message, Payload::Rsp(Rsp::default())));
                            }
                            return Err(ProcessError::NoSuchProviderType(req.type_name.clone()));
                        }
                        return Err(ProcessError::MissingFields("auth_info".to_string()));
                    }
                }
                MessageType::TestReq => {
                    if let Some(Payload::TestReq(req)) = payload {
                        let response = TestRsp {
                            test_data: format!("Echo: {}", req.test_data),
                        };
                        return Ok(wrap(&message, Payload::TestRsp(response)));
                    }
                }
                _ => {
                    return Err(ProcessError::UnknownMessageType(mtype.into()));
                }
            }
            return Err(ProcessError::UnexpectedPayload(mtype.into()));
        }
        WsMessage::Text(text) => {
            log::info!("{}", text);
            tx.send(WsMessage::Text(format!("Echo: {}", text).into()))
                .await
                .unwrap();
            return Err(ProcessError::None);
        }
        WsMessage::Ping(a) => {
            tx.send(WsMessage::Pong(a.clone())).await.unwrap();
            return Err(ProcessError::None);
        }
        _ => {
            return Err(ProcessError::None);
        }
    }
}

pub async fn handle_message(msg: WsMessage, ctx: Arc<BackendContext>) -> Result<()> {
    use msg::qcm_message::Payload;
    let mut id: Option<i32> = None;

    match process_message(&ctx, &ctx.ws_sender, &msg, &mut id).await {
        Ok(msg_rsp) => {
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
