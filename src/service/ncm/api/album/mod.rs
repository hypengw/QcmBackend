use crate::service::ncm::{
    model::{Album, Song},
    types::*,
};
use serde::{Deserialize, Serialize};

pub struct AlbumDetail {
    pub id: i64,
}

#[derive(Deserialize)]
pub struct AlbumDetailResponse {
    pub code: i64,
    pub album: Album,
    pub songs: Vec<Song>,
}

impl ApiInput for AlbumDetail {}

impl ApiModel for AlbumDetailResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for AlbumDetail {
    type Input = Self;
    type Output = AlbumDetailResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;
    
    fn path(&self) -> String {
        format!("/v1/album/{}", self.id)
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        Params::new()
    }
}

// Similar pattern for other album APIs...
