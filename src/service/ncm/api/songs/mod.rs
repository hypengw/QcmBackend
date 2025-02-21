use crate::service::ncm::{
    model::Song,
    types::*,
};
use serde::{Deserialize, Serialize};

pub struct SongDetail {
    pub ids: Vec<i64>,
}

#[derive(Deserialize)] 
pub struct SongDetailResponse {
    pub songs: Vec<Song>,
    pub privileges: Vec<Song::Privilege>,
    pub code: i64,
}

impl ApiInput for SongDetail {}

impl ApiModel for SongDetailResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from).map(|mut data: Self| {
            // Attach privileges to songs
            let n = data.songs.len().min(data.privileges.len());
            for i in 0..n {
                data.songs[i].privilege = Some(data.privileges[i].clone());
            }
            data
        })
    }
}

impl Api for SongDetail {
    type Input = Self;
    type Output = SongDetailResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;
    
    fn path(&self) -> String {
        "/v3/song/detail".to_string()
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        let mut params = Params::new();
        let ids_json = self.ids.iter()
            .map(|id| format!(r#"{{"id":{}}}"#, id))
            .collect::<Vec<_>>()
            .join(",");
        params.insert("c".to_string(), format!("[{}]", ids_json));
        params
    }
}

// More song APIs...
