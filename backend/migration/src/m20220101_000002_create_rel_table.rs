use qcm_core::model::*;
use sea_orm::Schema;
use sea_orm_migration::prelude::*;
use sea_query;

use crate::{unique_index, unique_index_name};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let builder = manager.get_database_backend();

        manager
            .create_table(
                Table::create()
                    .table(rel_album_artist::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(rel_album_artist::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(rel_album_artist::Column::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_album_artist::Column::AlbumId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_album_artist::Column::ArtistId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_album_artist::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_album_artist_library")
                            .from(
                                rel_album_artist::Entity,
                                rel_album_artist::Column::LibraryId,
                            )
                            .to(library::Entity, library::Column::LibraryId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_album_artist_album")
                            .from(rel_album_artist::Entity, rel_album_artist::Column::AlbumId)
                            .to(album::Entity, album::Column::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_album_artist_artist")
                            .from(rel_album_artist::Entity, rel_album_artist::Column::ArtistId)
                            .to(artist::Entity, artist::Column::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(unique_index!(
                rel_album_artist::Entity,
                rel_album_artist::Column::LibraryId,
                rel_album_artist::Column::AlbumId,
                rel_album_artist::Column::ArtistId
            ))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
            .await
    }
}
