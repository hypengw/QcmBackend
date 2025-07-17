use super::connection::{Connection, Creator, RemoteFileInfo, ResponceOneshot};
use crate::error::ProcessError;
use crate::http::range::HttpRange;
use qcm_core::model::type_enum::CacheType;

pub enum ReverseEvent {
    NewConnection(Connection, Creator, ResponceOneshot),
    NewRemoteFile(String, i64, CacheType, RemoteFileInfo, reqwest::Response),
    EndRemoteFile(i64),
    FinishFile(String, CacheType, RemoteFileInfo),
    EndConnection(i64),
    HasRemoteFile(i64),
    Stop,
}

pub fn wrap_creator<Fut>(
    ct: impl Fn(bool, Option<HttpRange>) -> Fut + Send + Sync + 'static,
) -> Creator
where
    Fut: std::future::Future<Output = Result<reqwest::Response, ProcessError>> + Send + 'static,
{
    Box::new(move |head: bool, r: Option<HttpRange>| Box::pin(ct(head, r)))
}
