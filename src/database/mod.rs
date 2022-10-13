use sqlx::postgres::{PgPool, PgPoolOptions};
use anyhow::Result;

pub mod twitch;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn init(url: &str) -> Result<Database> {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(url).await?;

        Ok(Database {
            pool
        })
    }
}
