use once_cell::sync::Lazy;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::plugin::Plugin;
use crate::provider::{Provider, ProviderMeta};

pub const APP_NAME: &str = "QcmBackend";
pub const APP_VERSION: &str = "0.1.0";

#[derive(Debug, Serialize, Deserialize)]
pub struct Setting {
    pub device_id: String,
}

impl Setting {
    fn new() -> Self {
        Self {
            device_id: Uuid::new_v4().to_string(),
        }
    }
}

pub struct Global {
    pub plugins: BTreeMap<String, Box<dyn Plugin>>,
    pub provider_metas: BTreeMap<String, ProviderMeta>,
    pub providers: BTreeMap<i64, Arc<dyn Provider>>,
    setting: Setting,
}

impl Global {
    fn new() -> Self {
        Self {
            plugins: BTreeMap::new(),
            provider_metas: BTreeMap::new(),
            providers: BTreeMap::new(),
            setting: Setting::new(),
        }
    }
}

static GLOBAL: Lazy<Arc<Mutex<Global>>> = Lazy::new(|| Arc::new(Mutex::new(Global::new())));

pub fn init(data_dir: &PathBuf) {
    let setting_path = data_dir.join("setting.json");
    let mut global = GLOBAL.lock().unwrap();

    if setting_path.exists() {
        let content = fs::read_to_string(&setting_path).expect("Failed to read setting.json");
        global.setting = serde_json::from_str(&content).expect("Failed to parse setting.json");
    } else {
        fs::create_dir_all(&data_dir).expect("Failed to create data directory");
        let setting = Setting::new();
        let content = serde_json::to_string_pretty(&setting).expect("Failed to serialize setting");
        fs::write(&setting_path, content).expect("Failed to write setting.json");
        global.setting = setting;
    }
}

pub async fn load_from_db(db: &DatabaseConnection) {
    use crate::model::provider;
    let providers = provider::Entity::find()
        .all(db)
        .await
        .expect("Failed to load providers");

    let mut global = GLOBAL.lock().unwrap();
    for provider_model in providers {
        if let Some(meta) = global.provider_metas.get(&provider_model.type_) {
            let provider = (meta.creator)(
                Some(provider_model.provider_id),
                &provider_model.name,
                &global.setting.device_id,
            );
            // TODO: not ignore
            let _ = provider.from_model(&provider_model);
            if let Some(id) = provider.id() {
                global.providers.insert(id, provider.clone());
            }
        }
    }
}

pub fn device_id() -> String {
    return GLOBAL.lock().unwrap().setting.device_id.clone();
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

pub fn provider_meta(type_name: &str) -> Option<ProviderMeta> {
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

pub fn with_provider_metas<F, R>(f: F) -> R
where
    F: FnOnce(&BTreeMap<String, ProviderMeta>) -> R,
{
    let global = GLOBAL.lock().unwrap();
    f(&global.provider_metas)
}

pub fn providers() -> Vec<Arc<dyn Provider>> {
    let g = GLOBAL.lock().unwrap();
    return g
        .providers
        .values()
        .cloned()
        .collect::<Vec<Arc<dyn Provider>>>();
}

pub fn add_provider(p: Arc<dyn Provider>) {
    let mut g = GLOBAL.lock().unwrap();
    g.providers.insert(p.id().unwrap(), p);
}
