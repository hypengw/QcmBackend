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
                    .col(ColumnDef::new(album::Column::SortName).string())
                    .col(
                        ColumnDef::new(album::Column::AddedTime)
                            .timestamp()
                            .not_null(),
                    )
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
                            .to(library::Entity, library::Column::LibraryId)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
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
                    .col(ColumnDef::new(artist::Column::SortName).string())
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
                            .to(library::Entity, library::Column::LibraryId)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
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
                    .col(ColumnDef::new(mix::Column::SortName).string())
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
                            .to(provider::Entity, provider::Column::ProviderId)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
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
                    .col(ColumnDef::new(song::Column::SortName).string())
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
                            .to(library::Entity, library::Column::LibraryId)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_song_album")
                            .from(song::Entity, song::Column::AlbumId)
                            .to(album::Entity, album::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
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
            .await?;

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
                            .name("fk_rel_album_artist_album")
                            .from(rel_album_artist::Entity, rel_album_artist::Column::AlbumId)
                            .to(album::Entity, album::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_album_artist_artist")
                            .from(rel_album_artist::Entity, rel_album_artist::Column::ArtistId)
                            .to(artist::Entity, artist::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(unique_index!(
                rel_album_artist::Entity,
                rel_album_artist::Column::AlbumId,
                rel_album_artist::Column::ArtistId
            ))
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(rel_song_artist::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(rel_song_artist::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(rel_song_artist::Column::SongId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_song_artist::Column::ArtistId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_song_artist::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_song_artist_song")
                            .from(rel_song_artist::Entity, rel_song_artist::Column::SongId)
                            .to(song::Entity, song::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_song_artist_artist")
                            .from(rel_song_artist::Entity, rel_song_artist::Column::ArtistId)
                            .to(artist::Entity, artist::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(unique_index!(
                rel_song_artist::Entity,
                rel_song_artist::Column::SongId,
                rel_song_artist::Column::ArtistId
            ))
            .await?;

        // Add rel_mix_song table creation
        manager
            .create_table(
                Table::create()
                    .table(rel_mix_song::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(rel_mix_song::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(rel_mix_song::Column::SongId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_mix_song::Column::MixId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_mix_song::Column::OrderIdx)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(rel_mix_song::Column::EditTime)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_mix_song_song")
                            .from(rel_mix_song::Entity, rel_mix_song::Column::SongId)
                            .to(song::Entity, song::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rel_mix_song_mix")
                            .from(rel_mix_song::Entity, rel_mix_song::Column::MixId)
                            .to(mix::Entity, mix::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(unique_index!(
                rel_mix_song::Entity,
                rel_mix_song::Column::MixId,
                rel_mix_song::Column::SongId
            ))
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
