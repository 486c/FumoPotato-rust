use sqlx::postgres::{PgPool, PgPoolOptions};
use eyre::Result;

pub mod twitch;
pub mod osu;

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

    pub async fn add_discord_channel(
        &self, 
        channel_id: i64
    ) -> Result<()> {

        sqlx::query!(
            "INSERT INTO discord_channels VALUES($1) ON CONFLICT DO NOTHING",
            channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }
}
