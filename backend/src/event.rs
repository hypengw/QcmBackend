pub use qcm_core::event::Event;
pub use tokio_tungstenite::tungstenite::Message as WsMessage;
use qcm_core::{self, provider};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub enum BackendEvent {
    Frist,
    NewProvider,
    End,
}

pub struct BackendContext {
    pub provider_context: Arc<provider::Context>,
    pub bk_ev_sender: Sender<BackendEvent>,
    pub ws_sender: Sender<WsMessage>,
}
