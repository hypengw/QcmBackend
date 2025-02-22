use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, SqlTable)]
#[table(name = "libraries")]
pub struct Library {
    #[column(primary_key)]
    pub library_id: i64,
    #[column(not_null)]
    pub name: String,
    #[column(not_null)]
    pub provider_id: i64,
    #[column(not_null)]
    pub native_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumRefer {
    pub item_id: String,
    pub library_id: i64,
    pub name: String,
    pub pic_url: String,
}

#[derive(Debug, Serialize, Deserialize, SqlTable)]
#[table(name = "albums")]
pub struct Album {
    #[column(primary_key)]
    pub item_id: String,
    #[column(foreign_key = "libraries(library_id)")]
    pub library_id: i64,
    #[column(not_null)]
    pub name: String,
    pub pic_url: String,
    pub publish_time: DateTime<Utc>,
    pub track_count: i32,
    pub description: String,
    pub company: String,
    pub album_type: String,
}

#[derive(Debug, Serialize, Deserialize, SqlTable)]
#[table(name = "artists")]
pub struct Artist {
    #[column(primary_key)]
    pub item_id: String,
    #[column(foreign_key = "libraries(library_id)")]
    pub library_id: i64,
    #[column(not_null)]
    pub name: String,
    pub pic_url: String,
    pub description: String,
    pub album_count: i32,
    pub music_count: i32,
    #[column(json)]
    pub alias: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MixRefer {
    pub item_id: i64,
    pub library_id: i64,
    pub name: String,
    pub pic_url: String,
    pub track_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mix {
    #[serde(flatten)]
    pub refer: MixRefer,
    pub special_type: i32,
    pub description: String,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub play_count: i32,
    pub user_id: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, SqlTable)]
#[table(name = "album_artist")]
pub struct AlbumArtist {
    pub library_id: i64,
    pub album_id: String,
    pub artist_id: String,
    pub edit_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, SqlTable)]
#[table(name = "song_artist")]
pub struct SongArtist {
    pub library_id: i64,
    pub song_id: String,
    pub artist_id: String,
    pub edit_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, SqlTable)]
#[table(name = "mix_song")]
pub struct MixSong {
    pub library_id: i64,
    pub song_id: String,
    pub mix_id: String,
    pub order_idx: Option<i32>,
    pub removed: bool,
    pub edit_time: DateTime<Utc>,
}
