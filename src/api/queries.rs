use crate::models::*;
use sqlx::SqlitePool;
use anyhow::Result;
use std::error::Error;

pub async fn get_albums_by_library(pool: &SqlitePool, library_id: i64) -> Result<Vec<Album>, Box<dyn Error>> {
    let albums = sqlx::query_as!(
        Album,
        r#"
        SELECT 
            item_id as "item_id: String",
            library_id, name, pic_url,
            publish_time, track_count, description,
            company, album_type
        FROM albums 
        WHERE library_id = ?
        "#,
        library_id
    )
    .fetch_all(pool)
    .await?;

    Ok(albums)
}

pub async fn get_artists_by_library(pool: &SqlitePool, library_id: i64) -> Result<Vec<Artist>, Box<dyn Error>> {
    let artists = sqlx::query_as!(
        Artist,
        r#"
        SELECT 
            item_id as "item_id: String",
            library_id, name, pic_url,
            description, album_count, music_count,
            alias
        FROM artists 
        WHERE library_id = ?
        "#,
        library_id
    )
    .fetch_all(pool)
    .await?;

    Ok(artists)
}

pub async fn get_songs_by_library(pool: &SqlitePool, library_id: i64) -> Result<Vec<Song>, Box<dyn Error>> {
    let songs = sqlx::query_as!(
        Song,
        "SELECT item_id as \"item_id: String\", name, album_id, artist_id, library_id FROM songs WHERE library_id = ?",
        library_id
    )
    .fetch_all(pool)
    .await?;

    Ok(songs)
}

pub async fn get_mixes_by_library(pool: &SqlitePool, library_id: i64) -> Result<Vec<Mix>, Box<dyn Error>> {
    let mixes = sqlx::query_as!(
        Mix,
        r#"
        SELECT 
            item_id as "item_id: String",
            library_id, name, pic_url,
            track_count, special_type, description,
            create_time, update_time, play_count,
            user_id, tags
        FROM mixes 
        WHERE library_id = ?
        "#,
        library_id
    )
    .fetch_all(pool)
    .await?;

    Ok(mixes)
}

pub async fn get_library(pool: &SqlitePool, library_id: i64) -> Result<Option<Library>, Box<dyn Error>> {
    let library = sqlx::query_as!(
        Library,
        "SELECT library_id, name, provider_id, native_id FROM libraries WHERE library_id = ?",
        library_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(library)
}

pub async fn get_library_data(pool: &SqlitePool, library_id: i64) -> Result<LibraryData> {
    let albums = Album::find_by_library_id(pool, library_id).await?;
    let artists = Artist::find_by_library_id(pool, library_id).await?;
    let mixes = Mix::find_by_library_id(pool, library_id).await?;
    
    Ok(LibraryData {
        albums,
        artists,
        mixes,
    })
}

pub async fn get_album_with_artists(pool: &SqlitePool, album_id: String) -> Result<(Album, Vec<Artist>)> {
    let album = Album::find_by_id(pool, album_id).await?
        .ok_or_else(|| anyhow::anyhow!("Album not found"))?;

    let artists = sqlx::query_as!(
        Artist,
        r#"
        SELECT a.*
        FROM artists a
        JOIN album_artist aa ON aa.artist_id = a.item_id
        WHERE aa.album_id = ?
        "#,
        album_id
    )
    .fetch_all(pool)
    .await?;

    Ok((album, artists))
}

// 其他复杂查询...
