use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

use qcm_core::db::fts::create_fts_table_and_triggers;

#[derive(DeriveMigrationName)]
pub struct Migration;

impl Migration {}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DROP TABLE IF EXISTS album_fts;")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS artist_fts;")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS song_fts;")
            .await?;

        Ok(())
    }
}
