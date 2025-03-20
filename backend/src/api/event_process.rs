use crate::event::{BackendContext, BackendEvent, Event};
use qcm_core::{self, provider};
use qcm_core::{global, Result};
use std::sync::Arc;

use crate::convert::*;
use crate::msg::{
    self, model::ProviderMeta, model::ProviderStatus, ProviderMetaStatusMsg, ProviderStatusMsg,
    QcmMessage,
};

pub async fn process_event(ev: Event, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        Event::End => return Ok(true),
    }
    return Ok(false);
}

pub async fn process_backend_event(ev: BackendEvent, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        BackendEvent::Frist => {
            send_provider_meta_status(ctx.as_ref()).await?;
            send_provider_status(ctx.as_ref()).await?;
        }
        BackendEvent::NewProvider => {
            send_provider_status(ctx.as_ref()).await?;
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

async fn send_provider_status(ctx: &BackendContext) -> Result<()> {
    let msg: Option<QcmMessage> = {
        let mut msg = ProviderStatusMsg::default();
        msg.statuses = global::providers()
            .iter()
            .map(|p| {
                let mut status = ProviderStatus::default();
                status.id = p.id().map_or(String::new(), |i| i.to_string());
                status.name = p.name();
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
    Ok(())
}
