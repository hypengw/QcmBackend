use qcm_core::{global::provider, model::*};
use sea_orm::Schema;
use sea_orm_migration::prelude::*;
use sea_query;

use crate::{unique_index, unique_index_name};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                sea_query::Index::create()
                    .name("rel_song_artist::ArtistId")
                    .table(rel_song_artist::Entity)
                    .col(rel_song_artist::Column::ArtistId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                sea_query::Index::create()
                    .name("rel_song_artist::SongId")
                    .table(rel_song_artist::Entity)
                    .col(rel_song_artist::Column::SongId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .name("rel_album_artist::AlbumId")
                    .table(rel_album_artist::Entity)
                    .col(rel_album_artist::Column::AlbumId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .name("rel_album_artist::ArtistId")
                    .table(rel_album_artist::Entity)
                    .col(rel_album_artist::Column::ArtistId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
