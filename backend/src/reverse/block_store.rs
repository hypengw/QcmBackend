use bytes::Bytes;
use qcm_core::model as sqlm;
use qcm_core::model::type_enum::CacheType;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use super::connection::RemoteFileInfo;
use super::io::IoCmd;

pub const BLOCK_SIZE: u64 = 8 * 1024 * 1024; // 8MB
pub const READ_CHUNK_SIZE: u64 = 64 * 1024; // 64KB per read

/// Block state machine
#[derive(Clone)]
pub enum BlockState {
    /// Being downloaded, partial data available
    Fetching {
        written: u64,
        notify: Arc<Notify>,
    },
    /// Fully cached on disk (and in DB)
    Cached,
}

/// Source-level metadata
#[derive(Clone, Debug)]
pub struct SourceMeta {
    pub content_type: String,
    pub content_length: u64,
    pub accept_ranges: bool,
    pub block_count: u32,
}

impl SourceMeta {
    pub fn from_remote_info(info: &RemoteFileInfo) -> Self {
        let content_length = info.full();
        Self {
            content_type: info.content_type.clone(),
            content_length,
            accept_ranges: info.accept_ranges,
            block_count: block_count(content_length),
        }
    }
}

/// Handle for an active SourceActor
pub struct SourceHandle {
    pub task: JoinHandle<()>,
}

/// Central block-based cache store, shared across all actors via Arc
pub struct BlockStore {
    /// source_key → source metadata
    sources: RwLock<HashMap<String, SourceMeta>>,
    /// block_key → block state (only for non-Cached blocks that are in memory;
    /// Cached blocks may or may not be present — DB is the source of truth)
    blocks: RwLock<HashMap<String, BlockState>>,
    /// source_key → active SourceActor handle
    active_sources: Mutex<HashMap<String, SourceHandle>>,
    /// IO thread command channel
    io_tx: std::sync::mpsc::Sender<IoCmd>,
    /// Cache directory
    cache_dir: PathBuf,
}

// --- Pure functions ---

pub fn block_index(offset: u64) -> u32 {
    (offset / BLOCK_SIZE) as u32
}

pub fn block_key(source_key: &str, index: u32) -> String {
    format!("{}_{}", source_key, index)
}

pub fn block_offset(index: u32) -> u64 {
    index as u64 * BLOCK_SIZE
}

pub fn block_count(content_length: u64) -> u32 {
    ((content_length + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32
}

pub fn block_path(cache_dir: &Path, key: &str) -> PathBuf {
    let prefix = key.get(0..2).unwrap_or("00");
    cache_dir.join(prefix).join(key)
}

pub fn block_path_downloading(cache_dir: &Path, key: &str) -> PathBuf {
    block_path(cache_dir, key).with_extension("downloading")
}

impl BlockStore {
    pub fn new(io_tx: std::sync::mpsc::Sender<IoCmd>, cache_dir: PathBuf) -> Self {
        Self {
            sources: RwLock::new(HashMap::new()),
            blocks: RwLock::new(HashMap::new()),
            active_sources: Mutex::new(HashMap::new()),
            io_tx,
            cache_dir,
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    // --- Source metadata ---

    pub fn get_source_meta(&self, source_key: &str) -> Option<SourceMeta> {
        self.sources.read().unwrap().get(source_key).cloned()
    }

    pub fn set_source_meta(&self, source_key: &str, meta: SourceMeta) {
        self.sources
            .write()
            .unwrap()
            .insert(source_key.to_string(), meta);
    }

    // --- Block state ---

    /// Query block state: memory first, then DB fallback
    pub async fn block_state(&self, bkey: &str, db: &DatabaseConnection) -> Option<BlockState> {
        // Check in-memory state
        {
            let blocks = self.blocks.read().unwrap();
            if let Some(state) = blocks.get(bkey) {
                return Some(state.clone());
            }
        }

        // Check DB
        if sqlm::cache_block::exists(db, bkey).await {
            let state = BlockState::Cached;
            self.blocks
                .write()
                .unwrap()
                .insert(bkey.to_string(), state.clone());
            return Some(state);
        }

        None
    }

    /// Mark a block as being fetched. Returns the Notify handle.
    pub fn begin_fetch(&self, bkey: &str) -> Arc<Notify> {
        let mut blocks = self.blocks.write().unwrap();
        // If already fetching, return existing notify
        if let Some(BlockState::Fetching { notify, .. }) = blocks.get(bkey) {
            return notify.clone();
        }
        let notify = Arc::new(Notify::new());
        blocks.insert(
            bkey.to_string(),
            BlockState::Fetching {
                written: 0,
                notify: notify.clone(),
            },
        );
        notify
    }

    /// Write data to a block (called by SourceActor)
    pub fn write_block_data(&self, bkey: &str, offset_in_block: u64, data: &Bytes) {
        // Send write command to IO thread
        let _ = self.io_tx.send(IoCmd::Write {
            key: bkey.to_string(),
            offset: offset_in_block,
            data: data.clone(),
        });

        // Update in-memory state and notify waiters
        let mut blocks = self.blocks.write().unwrap();
        if let Some(BlockState::Fetching { written, notify }) = blocks.get_mut(bkey) {
            let new_end = offset_in_block + data.len() as u64;
            if new_end > *written {
                *written = new_end;
            }
            notify.notify_waiters();
        }
    }

    /// Mark a block as fully cached
    pub async fn finish_block(
        &self,
        bkey: &str,
        source_key: &str,
        block_index: i32,
        block_size: i64,
        db: &DatabaseConnection,
    ) {
        // Rename .downloading → final
        let from = block_path_downloading(&self.cache_dir, bkey);
        let to = block_path(&self.cache_dir, bkey);
        let _ = self.io_tx.send(IoCmd::Rename {
            key: bkey.to_string(),
            from,
            to,
        });

        // Insert into DB
        use sea_orm::{EntityTrait, Set};
        let model = sqlm::cache_block::ActiveModel {
            key: Set(bkey.to_string()),
            source_key: Set(source_key.to_string()),
            block_index: Set(block_index),
            block_size: Set(block_size),
            blob: sea_orm::NotSet,
            timestamp: sea_orm::NotSet,
        };
        if let Err(e) = sqlm::cache_block::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(sqlm::cache_block::Column::Key)
                    .update_columns([
                        sqlm::cache_block::Column::BlockSize,
                    ])
                    .to_owned(),
            )
            .exec(db)
            .await
        {
            log::error!(target: "block_store", "finish_block DB error: {:?}", e);
        }

        // Update in-memory state: notify before changing to Cached
        {
            let blocks = self.blocks.read().unwrap();
            if let Some(BlockState::Fetching { notify, .. }) = blocks.get(bkey) {
                notify.notify_waiters();
            }
        }
        self.blocks
            .write()
            .unwrap()
            .insert(bkey.to_string(), BlockState::Cached);
    }

    /// Save source metadata to DB
    pub async fn finish_source(
        &self,
        source_key: &str,
        cache_type: CacheType,
        meta: &SourceMeta,
        db: &DatabaseConnection,
    ) {
        use sea_orm::{EntityTrait, Set};
        let model = sqlm::cache_source::ActiveModel {
            key: Set(source_key.to_string()),
            cache_type: Set(cache_type),
            content_type: Set(meta.content_type.clone()),
            content_length: Set(meta.content_length as i64),
            block_count: Set(meta.block_count as i32),
            timestamp: sea_orm::NotSet,
            last_use: sea_orm::NotSet,
        };
        if let Err(e) = sqlm::cache_source::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(sqlm::cache_source::Column::Key)
                    .update_columns([
                        sqlm::cache_source::Column::ContentType,
                        sqlm::cache_source::Column::ContentLength,
                        sqlm::cache_source::Column::BlockCount,
                    ])
                    .to_owned(),
            )
            .exec(db)
            .await
        {
            log::error!(target: "block_store", "finish_source DB error: {:?}", e);
        }
    }

    /// Wait until a block has data readable at the given offset
    pub async fn wait_readable(&self, bkey: &str, offset_in_block: u64) -> bool {
        loop {
            let state = {
                let blocks = self.blocks.read().unwrap();
                blocks.get(bkey).cloned()
            };
            match state {
                Some(BlockState::Cached) => return true,
                Some(BlockState::Fetching { written, notify }) => {
                    if written > offset_in_block {
                        return true;
                    }
                    // Wait for new data
                    notify.notified().await;
                }
                None => return false,
            }
        }
    }

    /// Get how many bytes are currently readable in a fetching block
    pub fn readable_bytes(&self, bkey: &str) -> u64 {
        let blocks = self.blocks.read().unwrap();
        match blocks.get(bkey) {
            Some(BlockState::Cached) => BLOCK_SIZE, // full block
            Some(BlockState::Fetching { written, .. }) => *written,
            None => 0,
        }
    }

    /// Read block data via IO thread
    pub async fn read_block(
        &self,
        bkey: &str,
        offset: u64,
        len: u64,
    ) -> Result<Bytes, anyhow::Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.io_tx.send(IoCmd::Read {
            key: bkey.to_string(),
            offset,
            len,
            reply: tx,
        })?;
        rx.await?
    }

    /// Create a new downloading file for a block
    pub fn create_block_file(&self, bkey: &str) {
        let _ = self.io_tx.send(IoCmd::CreateFile {
            key: bkey.to_string(),
        });
    }

    // --- Source actor management ---

    /// Check if a source needs a new SourceActor. Returns true if no active actor exists.
    pub fn needs_source_actor(&self, source_key: &str) -> bool {
        let sources = self.active_sources.lock().unwrap();
        !sources.contains_key(source_key)
    }

    /// Register a SourceActor task handle
    pub fn set_source_actor(&self, source_key: &str, task: JoinHandle<()>) {
        self.active_sources.lock().unwrap().insert(
            source_key.to_string(),
            SourceHandle { task },
        );
    }

    /// Remove a SourceActor registration
    pub fn remove_source_actor(&self, source_key: &str) {
        self.active_sources
            .lock()
            .unwrap()
            .remove(source_key);
    }

    /// Check if a source has an active SourceActor
    pub fn has_source_actor(&self, source_key: &str) -> bool {
        self.active_sources
            .lock()
            .unwrap()
            .contains_key(source_key)
    }
}
