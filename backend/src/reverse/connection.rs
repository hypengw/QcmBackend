use futures::channel::mpsc::Sender;
use qcm_core::Result;
use reqwest::{header::HeaderMap, Response};

pub struct Connection {
    key: String,
    tx: Sender<Vec<u8>>,
    cursor: usize,
    range_begin: usize,
    range_end: Option<usize>,
    started: bool,
    headers: HeaderMap,
    new_rsp: Box<dyn Fn(HeaderMap) -> Result<Response> + Send>,
}
