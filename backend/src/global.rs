use once_cell::sync::Lazy;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::event::BackendContext;

struct Global {
    pub contexts: BTreeMap<i64, Arc<BackendContext>>,
}

impl Global {
    fn new() -> Self {
        Self {
            contexts: BTreeMap::new(),
        }
    }
}

static GLOBAL: Lazy<Arc<Mutex<Global>>> = Lazy::new(|| Arc::new(Mutex::new(Global::new())));

pub fn context(port: i64) -> Option<Arc<BackendContext>> {
    let g = GLOBAL.lock().unwrap();
    g.contexts.get(&port).cloned()
}

pub fn reg_context(port: i64, c: Arc<BackendContext>) {
    let mut g = GLOBAL.lock().unwrap();
    g.contexts.insert(port, c);
}

pub fn unreg_context(port: i64) {
    let mut g = GLOBAL.lock().unwrap();
    g.contexts.remove(&port);
}
