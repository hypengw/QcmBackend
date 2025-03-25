use prost::{self, Message};
use qcm_core::{global, Result};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use qcm_core::model as sql_model;
use sea_orm::{sea_query, EntityTrait, PaginatorTrait};
use tokio::sync::mpsc::Sender;

use crate::api;
use crate::api::pagination::PageParams;
use crate::convert::QcmInto;
use crate::error::ProcessError;
use crate::event::{BackendContext, BackendEvent};
use crate::msg::{self, GetAlbumsRsp, GetProviderMetasRsp, MessageType, QcmMessage, Rsp, TestRsp};

type TX = Sender<WsMessage>;

fn wrap<T>(in_msg: &QcmMessage, msg: T) -> QcmMessage
where
    T: QcmInto<QcmMessage>,
{
    let mut qcm_msg: QcmMessage = msg.qcm_into();
    qcm_msg.id = in_msg.id;
    return qcm_msg;
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
                    return Ok(wrap(&message, response));
                }
                MessageType::AddProviderReq => {
                    if let Some(Payload::AddProviderReq(req)) = payload {
                        if let Some(auth_info) = &req.auth_info {
                            if let Some(meta) = qcm_core::global::provider_meta(&req.type_name) {
                                let provider =
                                    (meta.creator)(None, &req.name, &global::device_id());
                                provider
                                    .login(&ctx.provider_context, &auth_info.clone().qcm_into())
                                    .await?;

                                let id = api::db::add_provider(
                                    &ctx.provider_context.db,
                                    provider.clone(),
                                )
                                .await?;
                                provider.set_id(Some(id));

                                global::add_provider(provider.clone());

                                ctx.bk_ev_sender
                                    .send(BackendEvent::NewProvider { id })
                                    .await?;
                                return Ok(wrap(&message, Rsp::default()));
                            }
                            return Err(ProcessError::NoSuchProviderType(req.type_name.clone()));
                        }
                        return Err(ProcessError::MissingFields("auth_info".to_string()));
                    }
                }
                MessageType::GetAlbumsReq => {
                    if let Some(Payload::GetAlbumsReq(req)) = payload {
                        let page_params = PageParams::new(req.page, req.page_size);

                        let paginator = sql_model::album::Entity::find()
                            .paginate(&ctx.provider_context.db, page_params.page_size);

                        let total = paginator.num_items().await?;
                        let albums = paginator
                            .fetch_page(page_params.page)
                            .await?
                            .into_iter()
                            .map(|album| album.qcm_into())
                            .collect();

                        let rsp = GetAlbumsRsp {
                            albums,
                            total: total as i32,
                            has_more: page_params.has_more(total),
                        };
                        return Ok(wrap(&message, rsp));
                    }
                }
                MessageType::TestReq => {
                    if let Some(Payload::TestReq(req)) = payload {
                        let rsp = TestRsp {
                            test_data: format!("Echo: {}", req.test_data),
                        };
                        return Ok(wrap(&message, rsp));
                    }
                }
                _ => {
                    return Err(ProcessError::UnsupportedMessageType(mtype.into()));
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
