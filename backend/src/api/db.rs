use sqlx::sqlite::SqlitePool;
use std::error::Error;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, Box<dyn Error>> {
    let pool = SqlitePool::connect(database_url).await?;
    Ok(pool)
}
