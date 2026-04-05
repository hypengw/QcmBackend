use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use qcm_core::model::type_enum::CacheType;
use sea_orm::DatabaseConnection;

use super::block_store::{self, BlockStore, SourceMeta, BLOCK_SIZE};

/// SourceActor: manages a single upstream download for a source.
/// Streams the response body and splits it into fixed-size blocks,
/// writing each block to the BlockStore.
pub async fn source_actor(
    store: Arc<BlockStore>,
    db: DatabaseConnection,
    source_key: String,
    cache_type: CacheType,
    meta: SourceMeta,
    response: reqwest::Response,
    start_block: u32,
) {
    let mut stream = response.bytes_stream();
    let mut current_block = start_block;
    let mut offset_in_block: u64 = 0;

    // Create the first block file
    let first_key = block_store::block_key(&source_key, current_block);
    store.create_block_file(&first_key);
    store.begin_fetch(&first_key);

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                process_chunk(
                    &store,
                    &db,
                    &source_key,
                    cache_type,
                    &meta,
                    &chunk,
                    &mut current_block,
                    &mut offset_in_block,
                )
                .await;
            }
            Err(e) => {
                log::error!(target: "source_actor", "stream error for {}: {:?}", source_key, e);
                break;
            }
        }
    }

    // Finish the last block if it has data
    if offset_in_block > 0 {
        let bkey = block_store::block_key(&source_key, current_block);
        store
            .finish_block(
                &bkey,
                &source_key,
                current_block as i32,
                offset_in_block as i64,
                &db,
            )
            .await;
    }

    // Save source metadata to DB
    store
        .finish_source(&source_key, cache_type, &meta, &db)
        .await;

    // Unregister
    store.remove_source_actor(&source_key);
    log::debug!(target: "source_actor", "finished for {}", source_key);
}

async fn process_chunk(
    store: &BlockStore,
    db: &DatabaseConnection,
    source_key: &str,
    cache_type: CacheType,
    meta: &SourceMeta,
    chunk: &Bytes,
    current_block: &mut u32,
    offset_in_block: &mut u64,
) {
    let mut remaining = chunk.as_ref();

    while !remaining.is_empty() {
        let block_remaining = BLOCK_SIZE - *offset_in_block;
        let take = remaining.len().min(block_remaining as usize);
        let data = Bytes::copy_from_slice(&remaining[..take]);
        let bkey = block_store::block_key(source_key, *current_block);

        // Write to current block
        store.write_block_data(&bkey, *offset_in_block, &data);

        *offset_in_block += take as u64;
        remaining = &remaining[take..];

        // Block is full — finish it and start next
        if *offset_in_block >= BLOCK_SIZE {
            store
                .finish_block(
                    &bkey,
                    source_key,
                    *current_block as i32,
                    BLOCK_SIZE as i64,
                    db,
                )
                .await;

            *current_block += 1;
            *offset_in_block = 0;

            // Prepare next block
            let next_key = block_store::block_key(source_key, *current_block);
            store.create_block_file(&next_key);
            store.begin_fetch(&next_key);
        }
    }
}
