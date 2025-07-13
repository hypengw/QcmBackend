use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    future::BoxFuture,
};
use hyper::body::{Bytes, Frame};
use qcm_core::{model::type_enum::CacheType, Result};

use crate::http::range::{HttpContentRange, HttpRange};



pub struct Connection {
    pub key: String,
    pub range: Option<HttpRange>,
    pub cache_type: CacheType,
}

impl Connection {
    pub fn new(key: &str, range: Option<HttpRange>, cache_type: CacheType) -> Self {
        Self {
            key: key.to_string(),
            range: range,
            cache_type,
        }
    }

    pub fn start(&self, full: u64) -> u64 {
        match &self.range {
            Some(r) => r.start(full),
            None => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteFileInfo {
    pub content_type: String,
    pub content_length: u64,
    pub accept_ranges: bool,
    pub content_range: Option<HttpContentRange>,
}

impl RemoteFileInfo {
    pub fn full(&self) -> u64 {
        self.content_range
            .as_ref()
            .map(|cr| cr.full)
            .unwrap_or(self.content_length)
    }
}
