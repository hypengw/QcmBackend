use qcm_core::{db::fts::drop_fts_triggers, global::provider, model::*};
use sea_orm::Schema;
use sea_orm_migration::prelude::*;
use sea_query;

use crate::{unique_index, unique_index_name};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        drop_fts_triggers(db, "album").await?;
        drop_fts_triggers(db, "artist").await?;
        drop_fts_triggers(db, "song").await?;

        manager
            .drop_table(
                Table::drop()
                    .table(rel_album_artist::Entity)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(rel_song_artist::Entity)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(rel_mix_song::Entity)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(mix::Entity).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(song::Entity).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(album::Entity).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(artist::Entity).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(image::Entity).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(dynamic::Entity).if_exists().to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(remote_mix::Entity).if_exists().to_owned())
            .await?;

        Ok(())
    }
}
