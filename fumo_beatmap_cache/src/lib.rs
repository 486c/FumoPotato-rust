use std::sync::Arc;

use eyre::Result;
use metrics::counter;
use osu_api::OsuApi;
use sqlx::{PgPool, postgres::PgPoolOptions};

pub struct FumoBeatmapCache {
    pub osu_api: Arc<OsuApi>,
    pub pool: PgPool,
}

impl FumoBeatmapCache {
    pub async fn new(url: &str, osu_api: Arc<OsuApi>) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(4).connect(url).await?;

        Ok(Self { osu_api, pool })
    }

    async fn select_beatmap(&self, beatmap_id: i64) -> Result<Option<Vec<u8>>> {
        let res = sqlx::query_scalar!("SELECT data FROM beatmaps WHERE id = $1", beatmap_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(res)
    }

    async fn insert_beatmap(&self, beatmap_id: i64, data: &[u8]) -> Result<()> {
        sqlx::query!(
            "INSERT INTO beatmaps(id, data) VALUES ($1, $2)",
            beatmap_id,
            data
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Try to get a beatmap
    pub async fn get(&self, beatmap_id: i64) -> Result<Vec<u8>> {
        match self.select_beatmap(beatmap_id).await? {
            Some(v) => {
                counter!("beatmap_cache", "kind" => "hit").increment(1);

                Ok(v)
            }
            None => {
                counter!("beatmap_cache", "kind" => "miss").increment(1);
                let beatmap = self.osu_api.download_beatmap(beatmap_id).await?;
                self.insert_beatmap(beatmap_id, &beatmap).await?;
                Ok(beatmap)
            }
        }
    }
}
