use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

impl Migration {
    fn create_fts_table_sql(table_name: &str, columns: &[&str]) -> String {
        let columns_str = columns.join(",");
        format!(
            r#"CREATE VIRTUAL TABLE IF NOT EXISTS {table_name}_fts USING fts5 (
                {columns_str},
                content='{table_name}',
                content_rowid='id',
                tokenize='qcm'
            );"#,
        )
    }

    fn create_insert_trigger_sql(table_name: &str, columns: &[&str]) -> String {
        let columns_str = columns.join(", ");
        let values_str = columns
            .iter()
            .map(|col| format!("new.{}", col))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"CREATE TRIGGER {table_name}_fts_i AFTER INSERT ON {table_name} BEGIN
                INSERT INTO {table_name}_fts(rowid, {columns_str}) VALUES (new.id, {values_str});
            END;"#,
        )
    }

    fn create_delete_trigger_sql(table_name: &str, columns: &[&str]) -> String {
        let columns_str = columns.join(", ");
        let values_str = columns
            .iter()
            .map(|col| format!("old.{}", col))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"CREATE TRIGGER {table_name}_fts_d AFTER DELETE ON {table_name} BEGIN
                INSERT INTO {table_name}_fts({table_name}_fts, rowid, {columns_str}) VALUES('delete', old.id, {values_str});
            END;"#,
        )
    }

    fn create_update_trigger_sql(table_name: &str, columns: &[&str]) -> String {
        let columns_str = columns.join(", ");
        let old_values_str = columns
            .iter()
            .map(|col| format!("old.{}", col))
            .collect::<Vec<_>>()
            .join(", ");
        let new_values_str = columns
            .iter()
            .map(|col| format!("new.{}", col))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"CREATE TRIGGER {table_name}_fts_u AFTER UPDATE ON {table_name} BEGIN
                INSERT INTO {table_name}_fts({table_name}_fts, rowid, {columns_str}) VALUES('delete', old.id, {old_values_str});
                INSERT INTO {table_name}_fts(rowid, {columns_str}) VALUES (new.id, {new_values_str});
            END;"#
        )
    }

    async fn create_fts_table_and_triggers(
        manager: &SchemaManager<'_>,
        table_name: &str,
        columns: &[&str],
    ) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Create FTS table
        db.execute_unprepared(&Self::create_fts_table_sql(table_name, columns))
            .await?;

        // Create triggers
        db.execute_unprepared(&Self::create_insert_trigger_sql(table_name, columns))
            .await?;
        db.execute_unprepared(&Self::create_delete_trigger_sql(table_name, columns))
            .await?;
        db.execute_unprepared(&Self::create_update_trigger_sql(table_name, columns))
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        Self::create_fts_table_and_triggers(manager, "album", &["name", "description"]).await?;
        Self::create_fts_table_and_triggers(manager, "artist", &["name", "description"]).await?;
        Self::create_fts_table_and_triggers(manager, "song", &["name"]).await?;
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
