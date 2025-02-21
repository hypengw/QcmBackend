use std::collections::HashMap;
use reqwest::Response;
use serde::{Deserialize, Serialize};

pub type Params = HashMap<String, String>;
pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub enum IdType {
    Song = 0,
    Program,
    Album,
    Playlist,
    Djradio,
    Artist,
    User,
    Comment,
    Special,
}

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Get,
    Post,
}

#[derive(Debug, Clone, Copy)] 
pub enum CryptoType {
    Weapi,
    Eapi,
    None,
}

pub trait ApiModel: Sized {
    fn parse(response: Response, input: &impl ApiInput) -> Result<Self>;
}

pub trait ApiInput {}

pub trait Api {
    type Input: ApiInput;
    type Output: ApiModel;
    
    const OPERATION: Operation;
    const CRYPTO: CryptoType;
    
    fn path(&self) -> &str;
    fn query(&self) -> Params;
    fn body(&self) -> Params;
}
