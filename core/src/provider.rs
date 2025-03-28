use crate::{error::ConnectError, event::Event, Result};
use reqwest::Response;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    Username { username: String, pw: String },
    Phone { phone: String, pw: String },
    Email { email: String, pw: String },
    None,
}

impl Default for AuthMethod {
    fn default() -> Self {
        return AuthMethod::None;
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    pub db: DatabaseConnection,
    pub ev_sender: Sender<Event>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
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
/// - `device_id`: device id
///
pub type Creator = dyn Fn(Option<i64>, &str, &str) -> Arc<dyn Provider> + Send + Sync;

#[derive(Clone)]
pub struct ProviderMeta {
    pub type_name: String,
    pub svg: Arc<String>,
    pub creator: Arc<Creator>,
    pub mutable: bool,
    pub is_script: bool,
    pub has_server_url: bool,
}

impl ProviderMeta {
    pub fn new(type_name: &str, svg: Arc<String>, f: Arc<Creator>) -> Self {
        ProviderMeta {
            type_name: type_name.to_string(),
            svg,
            creator: f,
            mutable: false,
            is_script: false,
            has_server_url: true,
        }
    }
}

pub trait ProviderSession {
    fn load(&self, data: &str);
    fn save(&self) -> String;
}

#[async_trait::async_trait]
pub trait Provider: ProviderSession + Send + Sync {
    fn id(&self) -> Option<i64>;
    fn set_id(&self, id: Option<i64>);
    fn name(&self) -> String;
    fn type_name(&self) -> &str;
    fn from_model(&self, model: &crate::model::provider::Model) -> Result<()>;
    fn to_model(&self) -> crate::model::provider::ActiveModel;

    async fn login(&self, ctx: &Context, info: &AuthInfo) -> Result<()>;
    async fn sync(&self, ctx: &Context) -> Result<()>;
    async fn image(
        &self,
        ctx: &Context,
        item_id: &str,
        image_id: &str,
    ) -> Result<Response, ConnectError>;
}
