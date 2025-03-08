use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::plugin::Plugin;
use crate::provider::ProviderMeta;

pub struct Global {
    pub plugins: BTreeMap<String, Box<dyn Plugin>>,
    pub provider_metas: BTreeMap<String, ProviderMeta>,
}

impl Global {
    fn new() -> Self {
        Self {
            plugins: BTreeMap::new(),
            provider_metas: BTreeMap::new(),
        }
    }
}

static GLOBAL: Lazy<Arc<Mutex<Global>>> = Lazy::new(|| Arc::new(Mutex::new(Global::new())));

pub fn init() {
    Lazy::force(&GLOBAL);
}

pub fn add_plugin(p: Box<dyn Plugin>) {
    GLOBAL
        .lock()
        .unwrap()
        .plugins
        .insert((*p).id().to_string(), p);
}

pub fn reg_provider_meta(meta: ProviderMeta) {
    GLOBAL
        .lock()
        .unwrap()
        .provider_metas
        .insert(meta.type_name.clone(), meta);
}

pub fn provider_metas(type_name: &str) -> Option<ProviderMeta> {
    let g = GLOBAL.lock().unwrap();
    g.provider_metas.get(type_name).map(|s| s.clone())
}

pub fn plugin_for_each<F>(mut f: F)
where
    F: FnMut(&dyn Plugin),
{
    let g = GLOBAL.lock().unwrap();

    for p in &g.plugins {
        f(p.1.deref());
    }
}

pub fn with_plugins<F, R>(f: F) -> R
where
    F: FnOnce(&BTreeMap<String, Box<dyn Plugin>>) -> R,
{
    let global = GLOBAL.lock().unwrap();
    f(&global.plugins)
}
