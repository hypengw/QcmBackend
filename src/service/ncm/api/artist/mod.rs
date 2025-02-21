use crate::service::ncm::{
    model::{Artist, Song},
    types::*,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct ArtistDetail {
    pub id: i64,
}

#[derive(Deserialize)]
pub struct ArtistDetailResponse {
    pub code: i64,
    pub artist: Artist,
    pub hot_songs: Vec<Song>,
    pub more: bool,
}

impl ApiInput for ArtistDetail {}

impl ApiModel for ArtistDetailResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for ArtistDetail {
    type Input = Self; 
    type Output = ArtistDetailResponse;

    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;

    fn path(&self) -> String {
        format!("/v1/artist/{}", self.id)
    }

    fn query(&self) -> Params {
        Params::new()
    }

    fn body(&self) -> Params {
        Params::new()
    }
}
