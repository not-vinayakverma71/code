use sqlx::{Pool, Postgres};
use std::time::Duration;

pub struct DatabasePool {
    pool: Pool<Postgres>,
}

impl DatabasePool {
    /// Creates a new database connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(100)
            .min_connections(10)
            .connect_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;
        
        Ok(Self { pool })
    }
    
    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}