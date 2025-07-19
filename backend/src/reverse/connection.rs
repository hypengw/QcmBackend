use super::io::ReadState;
use crate::error::ProcessError;
use crate::http::{
    body_type::{ResponseBody, StreamItem as BodyStreamItem},
    range::{HttpContentRange, HttpRange},
};
use bytes::Bytes;
use futures::channel::mpsc::Sender as BoundedSender;
use http_body_util::StreamBody;
use qcm_core::model as sqlm;
use qcm_core::{model::type_enum::CacheType, Result};

pub enum ConnectionEvent {
    ReadedBuf(Bytes, ReadState),
    NoCache,
}

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
}

pub type ResponceOneshot =
    tokio::sync::oneshot::Sender<Result<hyper::Response<ResponseBody>, hyper::Error>>;

#[derive(Default, Clone, Debug, PartialEq)]
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

pub fn cache_to_remote_info(
    cache_info: &sqlm::cache::Model,
    range: &Option<HttpRange>,
) -> RemoteFileInfo {
    let full = cache_info.content_length as u64;
    match range {
        Some(range) => RemoteFileInfo {
            content_length: full - range.start(full),
            content_type: cache_info.content_type.clone(),
            accept_ranges: true,
            content_range: HttpContentRange::from_range(
                range.clone(),
                cache_info.content_length as u64,
            ),
        },
        None => RemoteFileInfo {
            content_length: full,
            content_type: cache_info.content_type.clone(),
            accept_ranges: true,
            content_range: None,
        },
    }
}

pub type Creator = Box<
    dyn Fn(
            bool,
            Option<HttpRange>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<reqwest::Response, ProcessError>> + Send>,
        > + Send
        + Sync
        + 'static,
>;

pub async fn real_request(
    ct: &Creator,
    head: bool,
    range: Option<HttpRange>,
) -> Result<(RemoteFileInfo, reqwest::Response), ProcessError> {
    let rsp = ct(head, range).await;
    match rsp {
        Ok(rsp) => {
            let headers = rsp.headers();
            let content_type = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string());
            let content_length: Option<u64> = headers
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            let content_range = headers
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| crate::http::range::parse_content_range(v));

            let accept_ranges = {
                if content_range.is_some() {
                    true
                } else {
                    headers
                        .get(reqwest::header::ACCEPT_RANGES)
                        .map(|v| v == "bytes")
                        .unwrap_or(false)
                }
            };

            match (content_type, content_length) {
                (Some(content_type), Some(content_length)) => Ok((
                    RemoteFileInfo {
                        content_type,
                        content_length,
                        accept_ranges,
                        content_range,
                    },
                    rsp,
                )),
                _ => Err(qcm_core::anyhow!("content_type/content_length not valid").into()),
            }
        }
        Err(e) => Err(e),
    }
}

pub fn create_rsp(
    id: i64,
    range: &Option<HttpRange>,
    info: &RemoteFileInfo,
) -> (hyper::Response<ResponseBody>, BoundedSender<BodyStreamItem>) {
    let (stream_tx, stream_rx) = futures::channel::mpsc::channel(4);
    let rsp = match (range, &info.content_range) {
        (Some(r), Some(cr)) => {
            if r.in_full(cr.full) {
                log::debug!(target: "reverse", "rsp({}) range: {}, content range: {}", id, r, cr);
                hyper::Response::builder()
                    .status(hyper::StatusCode::PARTIAL_CONTENT)
                    .header(hyper::header::CONTENT_TYPE, &info.content_type)
                    .header(hyper::header::CONTENT_LENGTH, info.content_length)
                    .header(hyper::header::ACCEPT_RANGES, "bytes")
                    .header(hyper::header::CONTENT_RANGE, cr.to_string())
                    .body(ResponseBody::BoundedStreamed(StreamBody::new(stream_rx)))
                    .unwrap()
            } else {
                log::debug!(target: "reverse", "rsp({}) range not satisfiable", id);
                hyper::Response::builder()
                    .status(hyper::StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(hyper::header::CONTENT_TYPE, &info.content_type)
                    .header(hyper::header::ACCEPT_RANGES, "bytes")
                    .header(hyper::header::CONTENT_RANGE, format!("bytes */{}", cr.full))
                    .body(ResponseBody::Empty)
                    .unwrap()
            }
        }
        (_, _) => {
            log::debug!(target: "reverse", "rsp({}) no range", id);
            let mut b = hyper::Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, &info.content_type)
                .header(hyper::header::CONTENT_LENGTH, info.content_length);

            if info.accept_ranges {
                b = b.header(hyper::header::ACCEPT_RANGES, "bytes");
            }

            b.body(ResponseBody::BoundedStreamed(StreamBody::new(stream_rx)))
                .unwrap()
        }
    };
    (rsp, stream_tx)
}
