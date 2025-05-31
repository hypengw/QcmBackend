use sea_orm_migration::prelude::*;

use qcm_core::model as sqlm;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create table
        manager
            .create_table(
                Table::create()
                    .table(sqlm::favorite::Entity)
                    .col(
                        ColumnDef::new(sqlm::favorite::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(sqlm::favorite::Column::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(sqlm::favorite::Column::ItemId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(sqlm::favorite::Column::ItemType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(sqlm::favorite::Column::EditTime)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp)),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-favorite-library_id")
                            .from(sqlm::favorite::Entity, sqlm::favorite::Column::LibraryId)
                            .to(sqlm::library::Entity, sqlm::library::Column::LibraryId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx-favorite-library_id")
                    .table(sqlm::favorite::Entity)
                    .col(sqlm::favorite::Column::LibraryId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-favorite-item")
                    .table(sqlm::favorite::Entity)
                    .col(sqlm::favorite::Column::ItemId)
                    .col(sqlm::favorite::Column::ItemType)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(sqlm::favorite::Entity).to_owned())
            .await
    }
}
