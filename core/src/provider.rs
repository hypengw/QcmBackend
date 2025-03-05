pub enum LoginInfo {
    Username { username: String, pw: String },
    Phone { username: String, pw: String },
    Email { email: String, pw: String },
}

pub trait SyncState {
    fn commit(&self, finished: i32, total: i32);
}

#[async_trait::async_trait]
pub trait Provider {
    async fn login(info: LoginInfo);
    async fn sync(state: &dyn SyncState);
}
