use crate::models::*;
use sqlx::SqlitePool;
use anyhow::Result;

pub async fn get_library_id(pool: &SqlitePool, provider_id: i64, native_id: &str) -> Result<Option<i64>> {
    let result = sqlx::query!(
        r#"
        SELECT library_id
        FROM libraries
        WHERE provider_id = ? AND native_id = ?
        "#,
        provider_id,
        native_id
    )
    .fetch_optional(pool)
    .await?
    .map(|row| row.library_id);

    Ok(result)
}

pub async fn get_library_id_list(pool: &SqlitePool) -> Result<Vec<i64>> {
    let results = sqlx::query!(
        r#"
        SELECT library_id 
        FROM libraries 
        ORDER BY library_id
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.library_id)
    .collect();

    Ok(results)
}

pub async fn get_library_list(pool: &SqlitePool) -> Result<Vec<Library>> {
    let libraries = sqlx::query_as!(
        Library,
        r#"
        SELECT 
            library_id, name, provider_id, native_id
        FROM libraries
        ORDER BY library_id
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(libraries)
}

pub async fn create_library(pool: &SqlitePool, library: Library) -> Result<Library> {
    let result = sqlx::query_as!(
        Library,
        r#"
        INSERT INTO libraries (name, provider_id, native_id)
        VALUES (?, ?, ?)
        RETURNING library_id, name, provider_id, native_id
        "#,
        library.name,
        library.provider_id,
        library.native_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result)
}

pub async fn delete_library(pool: &SqlitePool, library_id: i64) -> Result<bool> {
    let rows_affected = sqlx::query!(
        r#"
        DELETE FROM libraries 
        WHERE library_id = ?
        "#,
        library_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected > 0)
}

pub async fn batch_insert_libraries(pool: &SqlitePool, libraries: &[Library]) -> Result<()> {
    let mut tx = pool.begin().await?;

    for lib in libraries {
        sqlx::query!(
            r#"
            INSERT INTO libraries (name, provider_id, native_id)
            VALUES (?, ?, ?)
            ON CONFLICT (library_id) DO UPDATE SET
                name = excluded.name,
                provider_id = excluded.provider_id,
                native_id = excluded.native_id
            "#,
            lib.name,
            lib.provider_id,
            lib.native_id
        )
        .execute(&mut tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
