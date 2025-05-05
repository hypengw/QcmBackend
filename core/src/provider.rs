use crate::{
    error::ProviderError,
    event::Event,
    http,
    model::type_enum::{ImageType, ItemType},
    Result,
};
use reqwest::Response;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum AuthResult {
    Ok,
    Failed {
        message: String,
    },
    WrongPassword,
    NoSuchUsername,
    NoSuchEmail,
    NoSuchPhone,
    QrExpired,
    QrWaitScan,
    QrWaitComform {
        name: String,
        avatar_url: String,
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    Username { username: String, pw: String },
    Phone { phone: String, pw: String },
    Email { email: String, pw: String },
    Qr { key: String },
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
    pub cache_db: DatabaseConnection,
    pub ev_sender: Sender<Event>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct AuthInfo {
    pub server_url: String,
    pub method: AuthMethod,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct QrInfo {
    pub key: String,
    pub url: String,
}

/// Creator for provider
///
/// # Parameters
/// - `name`: provider name
/// - `device_id`: device id
///
pub type Creator = dyn Fn(Option<i64>, &str, &str) -> Result<Arc<dyn Provider>> + Send + Sync;

#[derive(Clone)]
pub struct ProviderMeta {
    pub type_name: String,
    pub svg: Arc<String>,
    pub creator: Arc<Creator>,
    pub mutable: bool,
    pub is_script: bool,
    pub has_server_url: bool,
    pub auth_types: Vec<i32>,
}

impl ProviderMeta {
    pub fn new(type_name: &str, auth_types: &[i32], svg: Arc<String>, f: Arc<Creator>) -> Self {
        ProviderMeta {
            type_name: type_name.to_string(),
            svg,
            creator: f,
            mutable: false,
            is_script: false,
            has_server_url: true,
            auth_types: auth_types.to_vec(),
        }
    }
}

pub trait ProviderSession {
    fn load_cookie(&self, data: &str);
    fn save_cookie(&self) -> String;
}

pub trait ProviderCommon {
    fn id(&self) -> Option<i64>;
    fn set_id(&self, id: Option<i64>);
    fn name(&self) -> String;
    fn set_name(&self, name: &str);
    fn type_name(&self) -> String;
    fn base_url(&self) -> String;
    fn auth_method(&self) -> Option<AuthMethod>;
    fn load_auth_info(&self, base_url: &str, auth_method: Option<AuthMethod>);
}

#[async_trait::async_trait]
pub trait Provider: ProviderCommon + ProviderSession + Send + Sync {
    fn load(&self, data: &str);
    fn save(&self) -> String;

    async fn check(&self, ctx: &Context) -> Result<(), ProviderError>;
    async fn auth(&self, ctx: &Context, info: &AuthInfo) -> Result<AuthResult, ProviderError>;
    async fn sync(&self, ctx: &Context) -> Result<(), ProviderError>;

    async fn qr(&self, _ctx: &Context) -> Result<QrInfo, ProviderError> {
        Err(ProviderError::NotImplemented)
    }

    async fn image(
        &self,
        ctx: &Context,
        item_id: &str,
        image_id: Option<&str>,
        image_type: ImageType,
    ) -> Result<Response, ProviderError>;

    async fn audio(
        &self,
        ctx: &Context,
        item_id: &str,
        headers: Option<http::HeaderMap>,
    ) -> Result<Response, ProviderError>;
}

struct ProviderCommonDataInner {
    id: Option<i64>,
    name: String,
    base_url: String,
    auth_method: Option<AuthMethod>,
}

pub struct ProviderCommonData {
    pub device_id: String,
    meta_type: String,
    inner: std::sync::RwLock<ProviderCommonDataInner>,
}

impl ProviderCommonData {
    pub fn new(id: Option<i64>, name: &str, device_id: &str, meta_type: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
            meta_type: meta_type.to_string(),
            inner: std::sync::RwLock::new(ProviderCommonDataInner {
                id: id,
                name: name.to_string(),
                base_url: String::new(),
                auth_method: None,
            }),
        }
    }
    pub fn id(&self) -> Option<i64> {
        self.inner.read().unwrap().id
    }
    fn set_id(&self, id: Option<i64>) {
        let mut inner = self.inner.write().unwrap();
        inner.id = id;
    }
}

pub trait HasCommonData {
    fn common<'a>(&'a self) -> &'a ProviderCommonData;
}

impl<T: HasCommonData> ProviderCommon for T {
    fn id(&self) -> Option<i64> {
        self.common().id()
    }
    fn set_id(&self, id: Option<i64>) {
        self.common().set_id(id);
    }
    fn name(&self) -> String {
        self.common().inner.read().unwrap().name.clone()
    }
    fn set_name(&self, name: &str) {
        let mut inner = self.common().inner.write().unwrap();
        inner.name = name.to_string();
    }
    fn type_name(&self) -> String {
        self.common().meta_type.clone()
    }
    fn base_url(&self) -> String {
        self.common().inner.read().unwrap().base_url.clone()
    }
    fn auth_method(&self) -> Option<AuthMethod> {
        self.common().inner.read().unwrap().auth_method.clone()
    }
    fn load_auth_info(&self, base_url: &str, auth_method: Option<AuthMethod>) {
        let mut inner = self.common().inner.write().unwrap();
        inner.base_url = base_url.to_string();
        inner.auth_method = auth_method;
    }
}
