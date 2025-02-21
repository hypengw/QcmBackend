use crate::service::ncm::{
    model::{Song, RadioProgram},
    types::*,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct RadioGet {
    pub mode: RadioMode,
    pub sub_mode: Option<RadioSubMode>,
    pub limit: Option<i32>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RadioMode {
    Default,
    Aidj,
    Familiar, 
    Explore,
    SceneRcmd,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")] 
pub enum RadioSubMode {
    Exercise,
    Focus,
    NightEmo,
}

#[derive(Deserialize)]
pub struct RadioGetResponse {
    pub code: i64,
    pub pop_adjust: bool,
    pub data: Vec<Song>,
}

impl ApiInput for RadioGet {}

impl ApiModel for RadioGetResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for RadioGet {
    type Input = Self;
    type Output = RadioGetResponse;

    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;

    fn path(&self) -> String {
        "/radio/get".into()
    }

    fn query(&self) -> Params {
        Params::new()
    }

    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("mode".into(), serde_json::to_string(&self.mode).unwrap());
        if let Some(sub_mode) = &self.sub_mode {
            params.insert("subMode".into(), serde_json::to_string(&sub_mode).unwrap());
        }
        if let Some(limit) = self.limit {
            params.insert("limit".into(), limit.to_string());
        }
        params
    }
}
