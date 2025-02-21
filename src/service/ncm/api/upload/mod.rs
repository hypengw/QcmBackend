use crate::service::ncm::{
    types::*,
    model::Song,
};
use serde::{Deserialize, Serialize};
use bytes::Bytes;

pub struct CloudUploadCheck {
    pub bitrate: String,
    pub ext: String,
    pub length: usize,
    pub md5: String,
    pub song_id: String,
    pub version: i64,
}

#[derive(Deserialize)]
pub struct CloudUploadCheckResponse {
    pub code: i64,
    pub song_id: String,
    pub need_upload: bool,
}

impl ApiInput for CloudUploadCheck {}

impl ApiModel for CloudUploadCheckResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for CloudUploadCheck {
    type Input = Self;
    type Output = CloudUploadCheckResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;
    
    fn path(&self) -> String {
        "/cloud/upload/check".to_string()
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("bitrate".to_string(), self.bitrate.clone());
        params.insert("ext".to_string(), self.ext.clone());
        params.insert("length".to_string(), self.length.to_string());
        params.insert("md5".to_string(), self.md5.clone());
        params.insert("songId".to_string(), self.song_id.clone());
        params.insert("version".to_string(), self.version.to_string());
        params
    }
}

pub struct CloudUpload {
    pub upload_host: String,
    pub bucket: String,
    pub object: String,
    pub token: String,
    pub md5: String,
    pub content_type: String,
    pub size: usize,
    pub data: Bytes,
}

#[derive(Deserialize)]
pub struct CloudUploadResponse {
    pub request_id: String,
    pub offset: i64,
}

// ... implement ApiInput, ApiModel and Api for CloudUpload ...

pub struct CloudUploadInfo {
    pub song_id: String,
    pub resource_id: String,
    pub md5: String,
    pub filename: String,
    pub song_name: String,
    pub album: String,
    pub artist: String,
    pub bitrate: i64,
}

#[derive(Deserialize)]
pub struct CloudUploadInfoResponse {
    pub code: i64,
    pub song_id: String,
}

// ... implement ApiInput, ApiModel and Api for CloudUploadInfo ...
