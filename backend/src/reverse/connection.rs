use std::future::Future;

use crate::error::HttpError;
use futures::{channel::mpsc::{UnboundedReceiver, UnboundedSender}, future::BoxFuture};
use http_range_header::parse_range_header;
use hyper::body::{Bytes, Frame};
use qcm_core::Result;
use reqwest::{header::HeaderMap, Response};
use std::pin::Pin;

type Creator =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<Response>>>> + Send + 'static>;

type Range = http_range_header::SyntacticallyCorrectRange;

pub fn parse_range(s: &str) -> Result<Range, HttpError> {
    match parse_range_header(s) {
        Ok(parsed) => {
            if parsed.ranges.len() == 1 {
                let r = parsed.ranges[0];
                Ok(r)
            } else {
                Err(HttpError::UnsupportedRange(s.to_string()))
            }
        }
        Err(_) => Err(HttpError::UnsupportedRange(s.to_string())),
    }
}

pub struct Connection {
    key: String,
    cursor: usize,
    range: Option<Range>,
    started: bool,
    new_rsp: Creator,
}

impl Connection {
    pub fn new<Fut>(
        key: &str,
        range: Option<Range>,
        new_rsp: impl Fn() -> Fut + Send + 'static,
    ) -> Self
    where
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        Self {
            key: key.to_string(),
            cursor: 0,
            range: range,
            started: false,
            new_rsp: Box::new(move || Box::pin(new_rsp())),
        }
    }
}
