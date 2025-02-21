use crate::service::ncm::{
    model::{Playlist, Song},
    types::*,
};
use serde::{Deserialize, Serialize};

pub struct PlaylistDetail {
    pub id: i64,
    pub n: i64,
    pub s: i64,
}

#[derive(Deserialize)]
pub struct PlaylistDetailResponse {
    pub code: i64,
    pub playlist: Playlist,
    #[serde(default)]
    pub privileges: Option<Vec<Song::Privilege>>,
}

impl ApiInput for PlaylistDetail {}

impl ApiModel for PlaylistDetailResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from).map(|mut data: Self| {
            // Attach privileges to tracks if available
            if let (Some(tracks), Some(privileges)) = (&mut data.playlist.tracks, &data.privileges) {
                let n = tracks.len().min(privileges.len());
                for i in 0..n {
                    tracks[i].privilege = Some(privileges[i].clone());
                }
            }
            data
        })
    }
}

impl Api for PlaylistDetail {
    type Input = Self;
    type Output = PlaylistDetailResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Eapi;
    
    fn path(&self) -> String {
        "/v6/playlist/detail".into()
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("id".to_string(), self.id.to_string());
        params.insert("n".to_string(), self.n.to_string());
        params.insert("s".to_string(), self.s.to_string());
        params
    }
}
