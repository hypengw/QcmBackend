use crate::event::{BackendContext, BackendEvent, Event};
use qcm_core::{self, provider};
use qcm_core::{global, Result};
use std::sync::Arc;

use crate::convert::*;
use crate::msg::{self, model::ProviderMeta, model::ProviderStatus, ProviderStatusMsg, QcmMessage};

pub async fn process_event(ev: Event, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        Event::End => return Ok(true),
    }
    return Ok(false);
}

pub async fn process_backend_event(ev: BackendEvent, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        BackendEvent::Frist => {
            send_provider_status(ctx.as_ref(), true).await?;
        }
        BackendEvent::NewProvider => {
            send_provider_status(ctx.as_ref(), false).await?;
        }
        BackendEvent::End => return Ok(true),
    }
    return Ok(false);
}

async fn send_provider_status(ctx: &BackendContext, has_meta: bool) -> Result<()> {
    let msg: QcmMessage = {
        let mut msg = ProviderStatusMsg::default();
        msg.statuses = global::providers()
            .iter()
            .map(|p| {
                let mut status = ProviderStatus::default();
                status.id = p.id().map_or(String::new(), |i| i.to_string());
                status.name = p.name();
                if has_meta {
                    status.meta = global::provider_meta(p.type_name()).map(|p| p.qcm_into());
                }
                return status;
            })
            .collect();
        msg.qcm_into()
    };
    ctx.ws_sender.send(msg.qcm_try_into()?).await?;
    Ok(())
}
