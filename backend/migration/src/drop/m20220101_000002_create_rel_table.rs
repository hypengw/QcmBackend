use qcm_core::model::*;
use sea_orm::Schema;
use sea_orm_migration::prelude::*;
use sea_query;

use crate::unique_index_name;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        rel_song_artist::Entity,
                        rel_song_artist::Column::SongId,
                        rel_song_artist::Column::ArtistId
                    ))
                    .table(rel_song_artist::Entity)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(rel_song_artist::Entity).to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        rel_album_artist::Entity,
                        rel_album_artist::Column::AlbumId,
                        rel_album_artist::Column::ArtistId
                    ))
                    .table(rel_album_artist::Entity)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(rel_album_artist::Entity).to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        rel_mix_song::Entity,
                        rel_mix_song::Column::MixId,
                        rel_mix_song::Column::SongId
                    ))
                    .table(rel_mix_song::Entity)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(rel_mix_song::Entity).to_owned())
            .await
    }
}
