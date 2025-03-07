use qcm_core::model::*;
use sea_orm::Schema;
use sea_orm_migration::{prelude::*, schema::*};
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
                    .col(
                        ColumnDef::new(provider::Column::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(provider::Column::Auth)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(provider::Column::Cookie)
                            .string()
                            .not_null(),
                    )
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
            .create_table(schema.create_table_from_entity(library::Entity))
            .await?;

        manager
            .create_table(schema.create_table_from_entity(album::Entity))
            .await?;
        manager
            .create_index(unique_index!(
                album::Entity,
                album::Column::ItemId,
                album::Column::LibraryId
            ))
            .await?;

        manager
            .create_table(schema.create_table_from_entity(artist::Entity))
            .await?;
        manager
            .create_index(unique_index!(
                artist::Entity,
                artist::Column::ItemId,
                artist::Column::LibraryId
            ))
            .await?;

        manager
            .create_table(schema.create_table_from_entity(mix::Entity))
            .await?;
        manager
            .create_index(unique_index!(
                mix::Entity,
                mix::Column::ItemId,
                mix::Column::LibraryId
            ))
            .await?;

        manager
            .create_table(schema.create_table_from_entity(radio::Entity))
            .await?;
        manager
            .create_index(unique_index!(
                radio::Entity,
                radio::Column::ItemId,
                radio::Column::LibraryId
            ))
            .await?;

        manager
            .create_table(schema.create_table_from_entity(song::Entity))
            .await?;
        manager
            .create_index(unique_index!(
                song::Entity,
                song::Column::ItemId,
                song::Column::LibraryId
            ))
            .await?;

        manager
            .create_table(schema.create_table_from_entity(program::Entity))
            .await?;
        manager
            .create_index(unique_index!(
                program::Entity,
                program::Column::ItemId,
                program::Column::LibraryId
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
                        program::Column::ItemId,
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
                        song::Column::ItemId,
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
                        radio::Entity,
                        radio::Column::ItemId,
                        radio::Column::LibraryId
                    ))
                    .table(radio::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        mix::Entity,
                        mix::Column::ItemId,
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
                        artist::Column::ItemId,
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
                        album::Column::ItemId,
                        album::Column::LibraryId
                    ))
                    .table(album::Entity)
                    .to_owned(),
            )
            .await?;

        // Drop tables
        manager
            .drop_table(Table::drop().table(program::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(song::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(radio::Entity).to_owned())
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
