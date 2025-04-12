use crate::task::TaskManagerOper;
pub use qcm_core::event::Event;
pub use qcm_core::event::SyncCommit;
use qcm_core::{self, provider};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
pub use tokio_tungstenite::tungstenite::Message as WsMessage;

pub enum BackendEvent {
    Frist,
    NewProvider { id: i64 },
    SyncCommit { id: i64, commit: SyncCommit },
    End,
}

pub struct BackendContext {
    pub provider_context: Arc<provider::Context>,
    pub bk_ev_sender: Sender<BackendEvent>,
    pub ws_sender: Sender<WsMessage>,
    pub oper: TaskManagerOper,
}
