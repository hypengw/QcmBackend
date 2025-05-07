use std::fmt;
use std::future::Future;

use crate::error::{HttpError, ProcessError};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    future::BoxFuture,
};
use http_range_header::parse_range_header;
use hyper::body::{Bytes, Frame};
use qcm_core::{model::type_enum::CacheType, Result};
use reqwest::{header::HeaderMap, Response};
use std::pin::Pin;

pub type Range = http_range_header::SyntacticallyCorrectRange;

#[derive(Clone, Debug, PartialEq)]
pub struct ContentRange {
    pub start: u64,
    pub end: u64,
    pub full: u64,
}

impl ContentRange {
    /// not check range if in full
    pub fn from_range(r: Range, full: u64) -> Option<ContentRange> {
        if full == 0 {
            None
        } else {
            Some(ContentRange {
                start: match r.start {
                    http_range_header::StartPosition::Index(offset) => offset,
                    http_range_header::StartPosition::FromLast(offset) => full - offset,
                },
                end: match r.end {
                    http_range_header::EndPosition::Index(offset) => offset,
                    http_range_header::EndPosition::LastByte => full - 1,
                },
                full,
            })
        }
    }
}

impl fmt::Display for ContentRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bytes {}-{}/{}", self.start, self.end, self.full)
    }
}

pub fn default_range() -> Range {
    Range {
        start: http_range_header::StartPosition::Index(0),
        end: http_range_header::EndPosition::LastByte,
    }
}

pub fn range_start(r: &Range, full: u64) -> u64 {
    match r.start {
        http_range_header::StartPosition::Index(pos) => pos,
        http_range_header::StartPosition::FromLast(pos) => full - pos,
    }
}

pub fn format_range(r: &Range) -> String {
    match (r.start, r.end) {
        (
            http_range_header::StartPosition::Index(start),
            http_range_header::EndPosition::Index(end),
        ) => {
            format!("bytes={}-{}", start, end)
        }
        (
            http_range_header::StartPosition::Index(start),
            http_range_header::EndPosition::LastByte,
        ) => {
            format!("bytes={}-", start)
        }
        (http_range_header::StartPosition::FromLast(start), _) => {
            format!("bytes=-{}", start)
        }
    }
}

pub fn range_in_full(r: &Range, full: u64) -> bool {
    match r.start {
        http_range_header::StartPosition::Index(offset) => {
            if full <= offset {
                return false;
            }
        }
        http_range_header::StartPosition::FromLast(offset) => {
            if full <= offset {
                return false;
            }
        }
    }
    match r.end {
        http_range_header::EndPosition::Index(offset) => {
            if full <= offset {
                return false;
            }
        }
        http_range_header::EndPosition::LastByte => {
            if full == 0 {
                return false;
            }
        }
    }
    return true;
}

pub fn parse_content_range(s: &str) -> Option<ContentRange> {
    let parts: Vec<&str> = s.split(" /").collect();
    match parts.as_slice() {
        ["bytes", range, full] => match range.split("-").collect::<Vec<&str>>().as_slice() {
            [start, end] => {
                if let (Ok(start), Ok(end), Ok(full)) = (start.parse(), end.parse(), full.parse()) {
                    Some(ContentRange { start, end, full })
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

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
    pub key: String,
    pub range: Option<Range>,
    pub cache_type: CacheType,
}

impl Connection {
    pub fn new(key: &str, range: Option<Range>, cache_type: CacheType) -> Self {
        Self {
            key: key.to_string(),
            range: range,
            cache_type,
        }
    }

    pub fn start(&self, full: u64) -> u64 {
        match self.range {
            Some(r) => range_start(&r, full),
            None => 0,
        }
    }
}
