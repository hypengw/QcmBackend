use crate::api::queries;
use futures_util::SinkExt;
use prost::Message;
use sqlx::SqlitePool;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub async fn handle_message(
    msg: WsMessage,
    pool: &SqlitePool,
    tx: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        WsMessage,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    if let WsMessage::Binary(data) = msg {
        let request = proto::Request::decode(&data[..])?;
        
        let mut response = proto::Response::default();
        
        match request.request_type.as_str() {
            "albums" => {
                let albums = queries::get_albums_by_library(pool, request.library_id).await?;
                response.albums = albums.into_iter().map(|a| proto::Album {
                    item_id: a.item_id,
                    name: a.name,
                }).collect();
            }
            "artists" => {
                let artists = queries::get_artists_by_library(pool, request.library_id).await?;
                response.artists = artists.into_iter().map(|a| proto::Artist {
                    item_id: a.item_id,
                    name: a.name,
                }).collect();
            }
            "songs" => {
                let songs = queries::get_songs_by_library(pool, request.library_id).await?;
                response.songs = songs.into_iter().map(|s| proto::Song {
                    item_id: s.item_id,
                    name: s.name,
                    album_id: s.album_id,
                    artist_id: s.artist_id,
                }).collect();
            }
            _ => return Ok(()),
        }

        let mut buf = Vec::new();
        response.encode(&mut buf)?;
        tx.send(WsMessage::Binary(buf)).await?;
    }
    Ok(())
}
