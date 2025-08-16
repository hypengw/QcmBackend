use qcm_core::{global::provider, model::{type_enum::AlbumType, *}};
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
            .alter_table(
                Table::alter()
                    .table(album::Entity)
                    .add_column(
                        ColumnDef::new(album::Column::Language)
                            .string()
                            .default("und")
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(album::Entity)
                    .add_column(
                        ColumnDef::new(album::Column::Duration)
                            .big_integer()
                            .default(0)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(album::Entity)
                    .add_column(
                        ColumnDef::new(album::Column::Type)
                            .integer()
                            .default(AlbumType::Album as i32)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
