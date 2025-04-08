use crate::event::{BackendContext, BackendEvent, Event};
use qcm_core::model as sqlm;
use qcm_core::provider::Provider;
use qcm_core::{self, provider};
use qcm_core::{global, Result};
use sea_orm::{EntityTrait, QueryFilter, QuerySelect};
use std::collections::BTreeMap;
use std::os::linux::raw::stat;
use std::sync::Arc;

use crate::convert::*;
use crate::msg::{
    self, model::ProviderMeta, model::ProviderStatus, ProviderMetaStatusMsg, ProviderStatusMsg,
    QcmMessage,
};

pub async fn process_event(ev: Event, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        Event::ProviderSync { id } => {
            if let Some(p) = global::provider(id) {
                ctx.oper.spawn({
                    let ctx = ctx.provider_context.clone();
                    async move {
                        match p.sync(&ctx).await {
                            Err(err) => {
                                log::error!("{:?}", err);
                            }
                            _ => {}
                        }
                        log::info!("sync ok");
                    }
                });
            }
        }
        Event::End => return Ok(true),
    }
    return Ok(false);
}

pub async fn process_backend_event(ev: BackendEvent, ctx: Arc<BackendContext>) -> Result<bool> {
    let ev_sender = &ctx.provider_context.ev_sender;
    match ev {
        BackendEvent::Frist => {
            send_provider_meta_status(ctx.as_ref()).await?;
            let providers = send_provider_status(ctx.as_ref()).await?;
            for p in providers {
                if let Some(id) = p.id() {
                    ev_sender.send(Event::ProviderSync { id: id }).await?;
                }
            }
        }
        BackendEvent::NewProvider { id } => {
            send_provider_status(ctx.as_ref()).await?;
            ev_sender.send(Event::ProviderSync { id }).await?;
        }
        BackendEvent::End => return Ok(true),
    }
    return Ok(false);
}

async fn send_provider_meta_status(ctx: &BackendContext) -> Result<()> {
    let msg: Option<QcmMessage> = {
        let mut msg = ProviderMetaStatusMsg::default();
        global::with_provider_metas(|metas| {
            for (_, v) in metas {
                msg.metas.push(v.clone().qcm_into());
            }
        });
        msg.full = true;
        if msg.metas.len() > 0 {
            Some(msg.qcm_into())
        } else {
            None
        }
    };
    if let Some(msg) = msg {
        ctx.ws_sender.send(msg.qcm_try_into()?).await?;
    }
    Ok(())
}

async fn send_provider_status(ctx: &BackendContext) -> Result<Vec<Arc<dyn Provider>>> {
    let db = &ctx.provider_context.db;
    let providers = global::providers();
    let msg: Option<QcmMessage> = {
        let mut msg = ProviderStatusMsg::default();

        let libraries = sqlm::library::Entity::find().all(db).await?;

        msg.statuses = providers
            .iter()
            .map(|p| {
                let mut status = ProviderStatus::default();
                status.id = p.id().unwrap_or(-1);
                status.name = p.name();
                status.type_name = p.type_name().to_string();

                for lib in &libraries {
                    if Some(lib.provider_id) == p.id() {
                        status.libraries.push(lib.clone().qcm_into());
                    }
                }
                return status;
            })
            .collect();

        msg.full = true;
        if msg.statuses.len() > 0 {
            Some(msg.qcm_into())
        } else {
            None
        }
    };
    if let Some(msg) = msg {
        ctx.ws_sender.send(msg.qcm_try_into()?).await?;
    }
    Ok(providers)
}
