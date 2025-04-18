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
        let schema = Schema::new(builder);

        manager
            .create_table(
                Table::create()
                    .table(provider::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(provider::Column::ProviderId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(provider::Column::Name).string().not_null())
                    .col(ColumnDef::new(provider::Column::Type).string().not_null())
                    .col(
                        ColumnDef::new(provider::Column::BaseUrl)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(provider::Column::AuthMethod).json())
                    .col(ColumnDef::new(provider::Column::Cookie).string().not_null())
                    .col(ColumnDef::new(provider::Column::Custom).string().not_null())
                    .col(
                        ColumnDef::new(provider::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(library::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(library::Column::LibraryId)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(library::Column::Name).string().not_null())
                    .col(
                        ColumnDef::new(library::Column::ProviderId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(library::Column::NativeId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(library::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(unique_index!(
                library::Entity,
                library::Column::ProviderId,
                library::Column::NativeId
            ))
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(album::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(album::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(album::Column::NativeId).string().not_null())
                    .col(
                        ColumnDef::new(album::Column::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(album::Column::Name).string().not_null())
                    .col(
                        ColumnDef::new(album::Column::PublishTime)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(album::Column::TrackCount)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(album::Column::Description)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(album::Column::Company).string().not_null())
                    .col(
                        ColumnDef::new(album::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_album_library")
                            .from(album::Entity, album::Column::LibraryId)
                            .to(library::Entity, library::Column::LibraryId),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(unique_index!(
                album::Entity,
                album::Column::NativeId,
                album::Column::LibraryId
            ))
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(artist::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(artist::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(artist::Column::NativeId).string().not_null())
                    .col(ColumnDef::new(artist::Column::Name).string().not_null())
                    .col(
                        ColumnDef::new(artist::Column::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(artist::Column::Description)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(artist::Column::AlbumCount)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(artist::Column::MusicCount)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(artist::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_artist_library")
                            .from(artist::Entity, artist::Column::LibraryId)
                            .to(library::Entity, library::Column::LibraryId),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(unique_index!(
                artist::Entity,
                artist::Column::NativeId,
                artist::Column::LibraryId
            ))
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(mix::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(mix::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(mix::Column::ProviderId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(mix::Column::Name).string().not_null())
                    .col(ColumnDef::new(mix::Column::NativeId).string().not_null())
                    .col(ColumnDef::new(mix::Column::TrackCount).integer().not_null())
                    .col(
                        ColumnDef::new(mix::Column::SpecialType)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(mix::Column::Description).string().not_null())
                    .col(ColumnDef::new(mix::Column::Tags).json().not_null())
                    .col(
                        ColumnDef::new(mix::Column::CreateTime)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(mix::Column::UpdateTime)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(mix::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mix_provider")
                            .from(mix::Entity, mix::Column::ProviderId)
                            .to(provider::Entity, provider::Column::ProviderId),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(unique_index!(
                mix::Entity,
                mix::Column::NativeId,
                mix::Column::ProviderId
            ))
            .await?;

        //        manager
        //            .create_table(schema.create_table_from_entity(radio::Entity))
        //            .await?;
        //        manager
        //            .create_index(unique_index!(
        //                radio::Entity,
        //                radio::Column::NativeId,
        //                radio::Column::LibraryId
        //            ))
        //            .await?;

        manager
            .create_table(
                Table::create()
                    .table(song::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(song::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(song::Column::NativeId).string().not_null())
                    .col(
                        ColumnDef::new(song::Column::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(song::Column::Name).string().not_null())
                    .col(ColumnDef::new(song::Column::AlbumId).big_integer())
                    .col(
                        ColumnDef::new(song::Column::TrackNumber)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(song::Column::DiscNumber)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(song::Column::Duration)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(song::Column::CanPlay).boolean().not_null())
                    .col(ColumnDef::new(song::Column::Tags).json().not_null())
                    .col(ColumnDef::new(song::Column::Popularity).double().not_null())
                    .col(
                        ColumnDef::new(song::Column::PublishTime)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(song::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_song_library")
                            .from(song::Entity, song::Column::LibraryId)
                            .to(library::Entity, library::Column::LibraryId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_song_album")
                            .from(song::Entity, song::Column::AlbumId)
                            .to(album::Entity, album::Column::Id),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(unique_index!(
                song::Entity,
                song::Column::NativeId,
                song::Column::LibraryId
            ))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop in reverse order - first indexes, then tables

        // Drop indexes
        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        program::Entity,
                        program::Column::NativeId,
                        program::Column::LibraryId
                    ))
                    .table(program::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        song::Entity,
                        song::Column::NativeId,
                        song::Column::LibraryId
                    ))
                    .table(song::Entity)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        mix::Entity,
                        mix::Column::NativeId,
                        mix::Column::LibraryId
                    ))
                    .table(mix::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        artist::Entity,
                        artist::Column::NativeId,
                        artist::Column::LibraryId
                    ))
                    .table(artist::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        album::Entity,
                        album::Column::NativeId,
                        album::Column::LibraryId
                    ))
                    .table(album::Entity)
                    .to_owned(),
            )
            .await?;

        //        manager
        //            .drop_index(
        //                Index::drop()
        //                    .name(unique_index_name!(
        //                        radio::Entity,
        //                        radio::Column::NativeId,
        //                        radio::Column::LibraryId
        //                    ))
        //                    .table(radio::Entity)
        //                    .to_owned(),
        //            )
        //            .await?;

        // manager
        //     .drop_table(Table::drop().table(program::Entity).to_owned())
        //     .await?;
        // manager
        //     .drop_table(Table::drop().table(radio::Entity).to_owned())
        //     .await?;

        // Drop tables
        manager
            .drop_table(Table::drop().table(song::Entity).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(mix::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(artist::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(album::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(library::Entity).to_owned())
            .await
    }
}
