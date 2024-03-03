use crate::database::Database;

use eyre::Result;


#[derive(sqlx::FromRow, Debug)]
pub struct OsuTrackedUser {
    pub osu_id: i64,
    pub channel_id: i64,
}

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
    
    /// Select osu tracking user based on
    /// provided channel_id and osu_user_id
    /// Usually used to check if user is already
    /// tracked in current channel
    pub async fn select_osu_tracking(
        &self,
        channel_id: i64,
        osu_user_id: i64,
    ) -> Result<Option<OsuTrackedUser>> {
        Ok(sqlx::query_as!(
            OsuTrackedUser,
            "SELECT * FROM osu_tracking WHERE channel_id = $1 AND osu_id = $2",
            channel_id, osu_user_id
        ).fetch_optional(&self.pool).await?)
    }
    
    /// Adds new user to the tracking
    pub async fn add_osu_tracking(
        &self,
        channel_id: i64,
        osu_user_id: i64,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO osu_tracking VALUES($1, $2)",
            osu_user_id, channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn remove_osu_tracking(
        &self,
        channel_id: i64,
        osu_user_id: i64,
    ) -> Result<()> {
        todo!()
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
