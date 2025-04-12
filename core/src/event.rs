use tokio::sync::oneshot;
pub enum SyncCommit {
    Start,
    AddAlbum(i32),
    AddArtist(i32),
    AddSong(i32),
    End,
}
pub enum Event {
    ProviderSync {
        id: i64,
        oneshot: Option<oneshot::Sender<i64>>,
    },
    SyncCommit {
        id: i64,
        commit: SyncCommit,
    },
    End,
}
