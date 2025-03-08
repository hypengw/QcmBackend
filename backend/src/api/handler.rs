use prost::Message;
use qcm_core::provider::Context;
use qcm_core::Result;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use tokio::sync::mpsc::Sender;

use crate::convert::QcmInto;
use crate::msg::{self, GetProviderMetasRsp, MessageType, QcmMessage, Rsp, TestRsp};

type TX = Sender<WsMessage>;

async fn send(tx: &TX, in_msg: &QcmMessage, playload: msg::qcm_message::Payload) -> Result<()> {
    let message = QcmMessage {
        r#type: in_msg.r#type() as i32,
        id: in_msg.id.clone(),
        payload: Some(playload),
    };

    let mut buf = Vec::new();
    message.encode(&mut buf)?;
    tx.send(WsMessage::Binary(buf.into())).await?;
    Ok(())
}

pub async fn handle_message(
    msg: WsMessage,
    ctx: Arc<Context>,
    tx: TX,
) -> Result<(), Box<dyn std::error::Error>> {
    use msg::qcm_message::Payload;
    if let WsMessage::Binary(data) = msg {
        let message = QcmMessage::decode(&data[..])?;
        let mtype = message.r#type();
        let payload = &message.payload;

        let mut err_payload = true;
        let rsp_base = Some(Rsp::default());

        match mtype {
            MessageType::GetProviderMetasReq => {
                err_payload = false;
                let response = GetProviderMetasRsp {
                    base: rsp_base,
                    metas: qcm_core::global::with_provider_metas(|metas| {
                        metas
                            .values()
                            .map(|el| -> msg::model::ProviderMeta { el.clone().qcm_into() })
                            .collect()
                    }),
                };

                send(&tx, &message, Payload::GetProviderMetasRsp(response)).await?;
            }
            MessageType::AddProviderReq => {
                if let Some(Payload::AddProviderReq(req)) = payload {
                    err_payload = false;
                    if let Some(p) = &req.provider {
                        if let (Some(meta), Some(auth_info)) =
                            (qcm_core::global::provider_meta(&p.type_name), &p.auth_info)
                        {
                            let provider = (meta.creator)(&p.name);
                            provider
                                .login(ctx.as_ref(), &auth_info.clone().qcm_into())
                                .await?;
                        }
                    }
                }
            }
            MessageType::TestReq => {
                if let Some(Payload::TestReq(req)) = payload {
                    err_payload = false;
                    let response = TestRsp {
                        base: rsp_base,
                        test_data: format!("Echo: {}", req.test_data),
                    };

                    send(&tx, &message, Payload::TestRsp(response)).await?;
                }
            }
            _ => {
                log::warn!("Unhandled message type: {:?}", mtype);
            }
        }
        if err_payload {
            log::warn!("Unexpected payload for message type: {:?}", mtype);
        }
    } else if let WsMessage::Text(text) = msg {
        log::info!("{}", text);
        tx.send(WsMessage::Text(format!("Echo: {}", text).into())).await?;
    }
    Ok(())
}
