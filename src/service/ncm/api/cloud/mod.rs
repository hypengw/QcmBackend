use crate::service::ncm::{
    model::UserCloudItem,
    types::*,
};
use serde::{Deserialize, Serialize};

pub struct UserCloud {
    pub offset: i64,
    pub limit: i64,
}

#[derive(Deserialize)]
pub struct UserCloudResponse {
    pub data: Vec<UserCloudItem>,
    pub count: i64,
    pub size: String,
    pub max_size: String,
    pub has_more: bool,
}

impl ApiInput for UserCloud {}

impl ApiModel for UserCloudResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for UserCloud {
    type Input = Self;
    type Output = UserCloudResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;
    
    fn path(&self) -> String {
        "/v1/cloud/get".into()
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("offset".to_string(), self.offset.to_string());
        params.insert("limit".to_string(), self.limit.to_string());
        params
    }
}
