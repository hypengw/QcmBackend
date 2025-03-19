use crate::event::{BackendContext, BackendEvent, Event};
use qcm_core::Result;
use qcm_core::{self, provider};
use std::sync::Arc;

use crate::msg::ProviderStatusMsg;

pub async fn process_event(ev: Event, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        Event::End => return Ok(true),
    }
    return Ok(false);
}

pub async fn process_backend_event(ev: BackendEvent, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        BackendEvent::Frist => {
            let mut msg = ProviderStatusMsg::default();
            // msg.
        }
        BackendEvent::NewProvider => {}
        BackendEvent::End => return Ok(true),
    }
    return Ok(false);
}
