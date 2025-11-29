use sea_orm_migration::prelude::*;

use crate::{unique_index, unique_index_name};
use qcm_core::model::{item, mix, remote_mix};

#[derive(DeriveMigrationName)]
pub struct Migration;

impl Migration {}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        manager
            .drop_index(
                Index::drop()
                    .name(unique_index_name!(
                        item::Entity,
                        item::Column::NativeId,
                        item::Column::Type,
                        item::Column::ProviderId
                    ))
                    .table(item::Entity)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        let stat = sea_orm::Statement::from_string(
            db.get_database_backend(),
            r#"
            CREATE UNIQUE INDEX item_native_type_provider_library_idx ON item (
                native_id,
                type,
                provider_id,
                IFNULL(library_id, -1)
            );
            "#,
        );
        db.execute(stat).await?;

        manager
            .drop_table(
                Table::drop()
                    .table(remote_mix::Entity)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(remote_mix::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(remote_mix::Column::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(remote_mix::Column::MixId).big_integer())
                    .col(ColumnDef::new(remote_mix::Column::Name).string().not_null())
                    .col(ColumnDef::new(remote_mix::Column::Description).string())
                    .col(ColumnDef::new(remote_mix::Column::MixType).string())
                    .col(ColumnDef::new(remote_mix::Column::TrackCount).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_remote_mix_mix")
                            .from(remote_mix::Entity, remote_mix::Column::MixId)
                            .to(mix::Entity, mix::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_remote_mix_item")
                            .from(remote_mix::Entity, remote_mix::Column::Id)
                            .to(item::Entity, item::Column::Id)
                            .on_delete(sea_query::ForeignKeyAction::Cascade),
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
