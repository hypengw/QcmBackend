use futures_util::SinkExt;
use prost::Message;
use qcm_core::provider::Context;
use qcm_core::Result;
use sea_orm::DatabaseConnection;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::convert::{QcmFrom, QcmInto};
use crate::msg::{self, GetProviderMetasRsp, MessageType, QcmMessage, Rsp, TestReq, TestRsp};

type TX = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    WsMessage,
>;

async fn send(tx: &mut TX, in_msg: &QcmMessage, playload: msg::qcm_message::Payload) -> Result<()> {
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
    tx: &mut TX,
) -> Result<(), Box<dyn std::error::Error>> {
    use msg::qcm_message::Payload;
    if let WsMessage::Binary(data) = msg {
        let message = QcmMessage::decode(&data[..])?;

        match message.r#type() {
            MessageType::GetProviderMetasReq => {
                let response = GetProviderMetasRsp {
                    base: Some(Rsp::default()),
                    metas: Vec::new(),
                };

                send(tx, &message, Payload::GetProviderMetasRsp(response)).await?;
            }
            MessageType::AddProviderReq => match &message.payload {
                Some(Payload::AddProviderReq(req)) => {
                    if let Some(p) = &req.provider {
                        if let (Some(meta), Some(auth_info)) =
                            (qcm_core::global::provider_meta(&p.type_name), &p.auth_info)
                        {
                            let provider = (meta.creator)(&p.name);
                            provider.login(ctx.as_ref(), &auth_info.clone().qcm_into()).await?;
                        }
                    }
                }
                _ => {
                    log::warn!("Unexpected payload for TEST message type");
                }
            },
            MessageType::TestReq => match &message.payload {
                Some(Payload::TestReq(req)) => {
                    let response = TestRsp {
                        base: Some(Rsp::default()),
                        test_data: format!("Echo: {}", req.test_data),
                    };

                    send(tx, &message, Payload::TestRsp(response)).await?;
                }
                _ => {
                    log::warn!("Unexpected payload for TEST message type");
                }
            },
            _ => {
                log::warn!("Unhandled message type: {:?}", message.r#type());
            }
        }
    } else if let WsMessage::Text(text) = msg {
        log::info!("{}", text);
    }
    Ok(())
}
