use tokio::sync::oneshot;

pub enum SyncState {
    Finished = 0,
    Syncing = 1,
    NotAuth = 2,
    NetworkError = 3,
    UknownError = 4,
}

pub enum SyncCommit {
    AddAlbum(i32),
    AddArtist(i32),
    AddSong(i32),
    SetState(SyncState),
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

impl From<crate::error::ProviderError> for SyncState {
    fn from(e: crate::error::ProviderError) -> Self {
        use crate::error::ProviderError;
        match e {
            ProviderError::NotAuth => SyncState::NotAuth,
            ProviderError::Request(_) => SyncState::NetworkError,
            e => SyncState::UknownError,
        }
    }
}
