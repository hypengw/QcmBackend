use sqlx::SqlitePool;
use crate::models::{Library, Album, Artist, Mix};

pub async fn setup_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create tables in the correct order (respect foreign keys)
    sqlx::query(&Library::create_table_sql())
        .execute(pool)
        .await?;

    sqlx::query(&Album::create_table_sql())
        .execute(pool)
        .await?;

    sqlx::query(&Artist::create_table_sql())
        .execute(pool)
        .await?;

    sqlx::query(&Mix::create_table_sql())
        .execute(pool)
        .await?;

    Ok(())
}
