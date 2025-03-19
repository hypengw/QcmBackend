pub use qcm_core::event::Event;
use qcm_core::Result;
use qcm_core::{self, provider};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
pub use tokio_tungstenite::tungstenite::Message as WsMessage;

pub enum BackendEvent {
    NewProvider,
    End,
}

pub struct BackendContext {
    pub provider_context: Arc<provider::Context>,
    pub bk_ev_sender: Sender<BackendEvent>,
    pub ws_sender: Sender<WsMessage>,
}

pub async fn process_event(ev: Event, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        Event::End => return Ok(true),
    }
    return Ok(false);
}

pub async fn process_backend_event(ev: BackendEvent, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        BackendEvent::NewProvider => {}
        BackendEvent::End => return Ok(true),
    }
    return Ok(false);
}
