use crate::service::ncm::{
    model::{Djradio, Program},
    types::*,
};
use serde::{Deserialize, Serialize};

pub struct DjradioDetail {
    pub id: i64,
}

#[derive(Deserialize)]
pub struct DjradioDetailResponse {
    pub data: Djradio,
}

impl ApiInput for DjradioDetail {}

impl ApiModel for DjradioDetailResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for DjradioDetail {
    type Input = Self;
    type Output = DjradioDetailResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;
    
    fn path(&self) -> String {
        "/djradio/v2/get".to_string()
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("id".to_string(), self.id.to_string());
        params
    }
}

pub struct DjradioPrograms {
    pub radio_id: i64,
    pub offset: i64,
    pub limit: i64,
    pub asc: bool,
}

#[derive(Deserialize)]
pub struct DjradioProgramsResponse {
    pub count: i64,
    pub more: bool,
    pub programs: Vec<Program>,
}

// ... implement ApiInput, ApiModel and Api for DjradioPrograms ...
