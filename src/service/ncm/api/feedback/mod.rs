use crate::service::ncm::{
    model::{SongId, ProgramId, PlaylistId, AlbumId, SpecialId, DjradioId},
    types::*,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct FeedbackWeblog {
    pub action: FeedbackAction,
    pub id: FeedbackId,
    pub content: String,
    pub time: Option<i64>,
    pub wifi: Option<i64>,
    pub download: Option<i64>,
    pub alg: Option<String>,
    pub source_id: Option<SourceId>,
}

#[derive(Serialize, Deserialize)]
pub enum FeedbackAction {
    Start,
    End,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeedbackId {
    Song(SongId),
    Program(ProgramId),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceId {
    Playlist(PlaylistId),
    Album(AlbumId),
    Special(SpecialId),
    Djradio(DjradioId),
}

#[derive(Deserialize)]
pub struct FeedbackWeblogResponse {
    pub code: i64,
    pub data: String,
}

impl ApiInput for FeedbackWeblog {}

impl ApiModel for FeedbackWeblogResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for FeedbackWeblog {
    type Input = Self;
    type Output = FeedbackWeblogResponse;

    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;

    fn path(&self) -> String {
        "/feedback/weblog".into()
    }

    fn query(&self) -> Params {
        Params::new()
    }

    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("action".into(), 
            match self.action {
                FeedbackAction::Start => "startplay".into(),
                FeedbackAction::End => "play".into(),
            }
        );
        // Add other params based on action type...
        params
    }
}
