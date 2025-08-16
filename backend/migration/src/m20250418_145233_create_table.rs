use crate::{unique_index, unique_index_name};
use qcm_core::model::image;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(image::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(image::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(image::Column::ItemId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(image::Column::ItemType).string().not_null())
                    .col(ColumnDef::new(image::Column::ImageType).string().not_null())
                    .col(
                        ColumnDef::new(image::Column::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(image::Column::NativeId).string())
                    .col(ColumnDef::new(image::Column::Db).string())
                    .col(ColumnDef::new(image::Column::Fresh).string().not_null())
                    .col(
                        ColumnDef::new(image::Column::Timestamp)
                            .timestamp()
                            .not_null(),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("idx-image-unique")
                            .col(image::Column::ItemId)
                            .col(image::Column::ItemType)
                            .col(image::Column::ImageType),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(unique_index!(
                image::Entity,
                image::Column::ItemId,
                image::Column::ItemType,
                image::Column::ImageType
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(image::Entity).to_owned())
            .await?;
        Ok(())
    }
}
