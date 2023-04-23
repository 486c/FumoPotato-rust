use crate::database::Database;

use eyre::Result;

#[derive(sqlx::FromRow, Debug)]
pub struct OsuDbUser {
    pub osu_id: i64,
    pub discord_id: i64,
}

impl Database {
    pub async fn get_osu_db_user(
        &self,
        discord_id: i64
    ) -> Result<Option<OsuDbUser>> {
        Ok(sqlx::query_as!(
            OsuDbUser,
            "SELECT * FROM osu_users WHERE discord_id = $1",
            discord_id
        ).fetch_optional(&self.pool).await?)
    }

    pub async fn link_osu(
        &self,
        discord_id: i64,
        osu_id: i64
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO osu_users(discord_id, osu_id) VALUES($1, $2)",
            discord_id, osu_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn unlink_osu(
        &self,
        discord_id: i64,
        ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM osu_users WHERE discord_id = $1",
            discord_id
            ).execute(&self.pool).await?;

        Ok(())
    }

}
