use crate::service::ncm::{
    model::{Comment, CommentId},
    types::*,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Comments {
    pub id: CommentId,
    pub type_: CommentType,
    pub offset: i32,
    pub limit: i32,
    pub before_time: i64,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub enum CommentType {
    Album = 0,
    Playlist = 1,
    Song = 2,
    Program = 3,
}

#[derive(Deserialize)]
pub struct CommentsResponse {
    pub code: i64,
    pub top_comments: Vec<Comment>,
    #[serde(default)]
    pub hot_comments: Option<Vec<Comment>>,
    pub comments: Vec<Comment>,
    pub total: i64,
    pub more_hot: Option<bool>,
    pub more: bool,
}

impl ApiInput for Comments {}

impl ApiModel for CommentsResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for Comments {
    type Input = Self;
    type Output = CommentsResponse;

    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;

    fn path(&self) -> String {
        let prefix = match self.type_ {
            CommentType::Album => "R_AL_3_",
            CommentType::Playlist => "A_PL_0_",
            CommentType::Song => "R_SO_4_",
            CommentType::Program => "A_DJ_1_",
        };
        format!("/v1/resource/comments/{}{}", prefix, self.id)
    }

    fn query(&self) -> Params {
        Params::new()
    }

    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("rid".into(), self.id.to_string());
        params.insert("limit".into(), self.limit.to_string());
        params.insert("offset".into(), self.offset.to_string());
        params.insert("beforeTime".into(), self.before_time.to_string());
        params
    }
}
