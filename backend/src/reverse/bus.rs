use super::connection::{ConnectionEvent, RemoteFileInfo};
use super::io::ReadState;
use crate::http::piece;
use bytes::Bytes;
use qcm_core::model::type_enum::CacheType;
use tokio::sync::mpsc::Sender;

pub enum EventBus {
    // from cnn
    // update io reader state and make io send readedbuf
    RequestRead(
        String, // cnn key
        i64,    // cnn id
        u64,    // cursor
        bool,   // has cache entry
    ),
    ReadContinue(i64 /* cnn id */),
    NewRemoteFile(String, i64, CacheType, RemoteFileInfo, reqwest::Response),

    // from cnn process
    NewConnection(
        i64,                     // cnn id
        Sender<ConnectionEvent>, // cnn event sender
    ),
    EndConnection(i64 /* cnn id */),

    // from io
    ReadedBuf(i64, Bytes, ReadState),
    DoRead, // make io loop
    FinishFile(String /*key */, CacheType, RemoteFileInfo),
    NoCache(i64),

    // from reverse
    DbFinishFile(String /*key */),
    EndRemoteFile(i64),
    NoRemoteFile(i64),
}
