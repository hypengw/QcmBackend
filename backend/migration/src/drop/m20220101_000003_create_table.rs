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
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(rel_song_artist::Entity).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(rel_album_artist::Entity).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(rel_mix_song::Entity).to_owned())
            .await?;

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
