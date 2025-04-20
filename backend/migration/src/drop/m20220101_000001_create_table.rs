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
                        mix::Column::ProviderId
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

        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        library::Entity,
                        library::Column::ProviderId,
                        library::Column::NativeId
                    ))
                    .table(library::Entity)
                    .to_owned(),
            )
            .await?;

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
        Ok(())
    }
}
