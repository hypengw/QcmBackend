use crate::Result;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    Username { username: String, pw: String },
    Phone { phone: String, pw: String },
    Email { email: String, pw: String },
    None,
}

#[derive(Clone, Debug)]
pub struct Context {
    pub db: DatabaseConnection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthInfo {
    pub server_url: String,
    pub method: AuthMethod,
}

pub trait SyncState {
    fn commit(&self, finished: i32, total: i32);
}

/// Creator for provider
///
/// # Parameters
/// - `name`: provider name
///
pub type Creator = dyn Fn(&str) -> Box<dyn Provider> + Send + Sync;

#[derive(Clone)]
pub struct ProviderMeta {
    pub type_name: String,
    pub mutable: bool,
    pub is_script: bool,
    pub creator: Arc<Creator>,
}

impl ProviderMeta {
    pub fn new(type_name: &str, f: Arc<Creator>) -> Self {
        ProviderMeta {
            type_name: type_name.to_string(),
            mutable: false,
            is_script: false,
            creator: f,
        }
    }
}

pub trait ProviderSession {
    fn load(&self, data: &str);
    fn save(&self) -> String;
}

#[async_trait::async_trait]
pub trait Provider: ProviderSession {
    fn id(&self) -> Option<i64>;
    fn name(&self) -> String;
    fn type_name(&self) -> &str;
    async fn login(&self, ctx: &Context, info: AuthInfo) -> Result<()>;
    async fn sync(&self, ctx: &Context, state: &dyn SyncState) -> Result<()>;
}
