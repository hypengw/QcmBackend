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
                    .table(sqlm::dynamic::Entity)
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::LibraryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::ItemId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::ItemType)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::IsExternal)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::PlayCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::IsFavorite)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::LastPosition)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(sqlm::dynamic::Column::EditTime)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp)),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-dynamic-library_id")
                            .from(sqlm::dynamic::Entity, sqlm::dynamic::Column::LibraryId)
                            .to(sqlm::library::Entity, sqlm::library::Column::LibraryId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-dynamic-item")
                    .table(sqlm::dynamic::Entity)
                    .col(sqlm::dynamic::Column::ItemId)
                    .col(sqlm::dynamic::Column::ItemType)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(sqlm::dynamic::Entity).to_owned())
            .await
    }
}
