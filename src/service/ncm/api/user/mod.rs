use crate::service::ncm::{
    model::UserProfile,
    types::*,
};
use serde::{Deserialize, Serialize};

pub struct UserAccount {}

#[derive(Deserialize)]
pub struct UserAccountResponse {
    pub code: i64,
    pub profile: Option<UserProfile>,
}

impl ApiInput for UserAccount {}

impl ApiModel for UserAccountResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for UserAccount {
    type Input = Self;
    type Output = UserAccountResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;
    
    fn path(&self) -> String {
        "/nuser/account/get".into()
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        Params::new()
    }
}
