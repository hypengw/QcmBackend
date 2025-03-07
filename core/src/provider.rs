use crate::Result;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    Username { username: String, pw: String },
    Phone { phone: String, pw: String },
    Email { email: String, pw: String },
    None
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

#[async_trait::async_trait]
pub trait Provider {
    fn id(&self) -> Option<i64>;
    async fn login(&self, ctx: &Context, info: AuthInfo) -> Result<()>;
    async fn sync(&self, ctx: &Context, state: &dyn SyncState) -> Result<()>;
}
