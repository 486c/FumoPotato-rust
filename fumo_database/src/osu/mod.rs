use crate::Database;

use chrono::{DateTime, NaiveDateTime, Utc};
use eyre::{Result, eyre};
use osu_api::models::{osu_matches::OsuMatchGame, OsuScore};

#[derive(sqlx::FromRow, Debug)]
pub struct OsuLinkedTrackedUser {
    pub osu_id: i64,
    pub channel_id: i64,
    pub osu_username: String,
}

#[derive(sqlx::FromRow, Debug)]
pub struct OsuTrackedUser {
    pub osu_id: i64,
    pub last_checked: NaiveDateTime,
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

    pub async fn add_osu_player(
        &self,
        osu_id: i64,
        username: &str,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO osu_players VALUES($1, $2) 
            ON CONFLICT DO NOTHING",
            osu_id, username
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn select_osu_tracking_by_channel(
        &self,
        channel_id: i64,
    ) -> Result<Vec<OsuLinkedTrackedUser>> {
        Ok(sqlx::query_as!(
            OsuLinkedTrackedUser,
            "select ot.osu_id, ot.channel_id, op.osu_username
            from osu_tracking ot 
            inner join osu_players op 
            on ot.osu_id = op.osu_id where channel_id = $1",
            channel_id 
        ).fetch_all(&self.pool).await?)
    }

    /// Select osu tracking user based on
    /// provided channel_id and osu_user_id
    /// Usually used to check if user is already
    /// tracked in current channel
    pub async fn select_osu_tracking(
        &self,
        channel_id: i64,
        osu_user_id: i64,
    ) -> Result<Option<OsuLinkedTrackedUser>> {
        Ok(sqlx::query_as!(
            OsuLinkedTrackedUser,
            "select ot.osu_id, ot.channel_id, op.osu_username
            from osu_tracking ot 
            inner join osu_players op 
            on ot.osu_id = op.osu_id 
            where channel_id = $1
            AND ot.osu_id = $2",
            channel_id, osu_user_id
        ).fetch_optional(&self.pool).await?)
    }

    pub async fn update_tracked_user_status(
        &self,
        osu_id: i64,
        last_checked: NaiveDateTime,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE osu_tracked_users
            SET last_checked = $1
            WHERE osu_id = $2",
            last_checked, osu_id
        ).execute(&self.pool).await?;

        Ok(())
    }
    
    pub async fn select_osu_tracked_linked_channels(
        &self,
        osu_id: i64
    ) -> Result<Vec<OsuLinkedTrackedUser>> {
        Ok(sqlx::query_as!(
            OsuLinkedTrackedUser,
            "select ot.osu_id, ot.channel_id, op.osu_username
            from osu_tracking ot 
            inner join osu_players op 
            on ot.osu_id = op.osu_id where ot.osu_id = $1",
            osu_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn select_osu_tracked_users(
        &self,
    ) -> Result<Vec<OsuTrackedUser>> {
        Ok(sqlx::query_as!(
            OsuTrackedUser,
            "SELECT * FROM osu_tracked_users"
        ).fetch_all(&self.pool).await?)
    }

    pub async fn remove_all_osu_tracking(
        &self,
        channel_id: i64,
    ) -> Result<()> {
        sqlx::query!("
            DELETE FROM osu_tracking WHERE channel_id = $1
        ", channel_id).execute(&self.pool).await?;

        Ok(())
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

    pub async fn add_tracked_osu_user(
        &self,
        osu_user_id: i64
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO osu_tracked_users 
            VALUES($1, now() at time zone('utc'))
            ON CONFLICT DO NOTHING
            ",
            osu_user_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn remove_osu_tracking(
        &self,
        channel_id: i64,
        osu_user_id: i64,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM osu_tracking WHERE
            osu_id = $1 and channel_id = $2",
            osu_user_id, channel_id
        ).execute(&self.pool).await?;

        Ok(())
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

    pub async fn is_osu_match_not_found(
        &self,
        match_id: i64
    ) -> Result<bool> {
        Ok(sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM osu_match_not_found WHERE id = $1) as "exists!""#,
            match_id
        ).fetch_one(&self.pool).await?)
    }

    pub async fn is_osu_match_exists(
        &self,
        match_id: i64,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM osu_matches WHERE id = $1) as "exists!""#,
            match_id
        ).fetch_one(&self.pool).await?)
    }

    pub async fn insert_osu_match_not_found (
        &self,
        match_id: i64
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO osu_match_not_found VALUES ($1)",
            match_id,
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn insert_osu_match(
        &self,
        match_id: i64,
        name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO osu_matches 
            (id, name, start_time, end_time)
            VALUES($1, $2, $3, $4)
            ",
            match_id,
            name,
            start_time.naive_utc(),
            end_time.naive_utc()
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn insert_osu_match_game_from_request(
        &self,
        match_id: i64,
        game: &OsuMatchGame
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO osu_match_games
            (id, match_id, beatmap_id, mods, mode, scoring_kind, team_kind, start_time, end_time)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            game.id,
            match_id,
            game.beatmap_id,
            game.mods.bits() as i64,
            game.mode.as_str(),
            game.scoring_kind.as_u8() as i16,
            game.team_kind.as_u8() as i16,
            game.start_time.naive_utc(),
            game.end_time.unwrap_or(
                DateTime::from_timestamp(0, 0).unwrap()
            ).naive_utc()
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn insert_osu_match_game_score_from_request(
        &self,
        match_id: i64,
        game_id: i64,
        beatmap_id: i64,
        score: &OsuScore
    ) -> Result<()> {
        if score.osu_match.is_none() {
            return Err(eyre!("Provided osu score doesn't contain match info")); // TODO: This should be here
        };

        let osu_match = &score.osu_match;
        let detail = osu_match.as_ref().unwrap();

        sqlx::query!(
            "INSERT INTO osu_match_game_scores
            (
                game_id, match_id, beatmap_id, user_id, accuracy, mods, score, 
                count50, count100, count300, countgeki, countkatu, countmiss, max_combo,
                slot, team, pass, pp
            )
            VALUES(
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NULL
            )",
            game_id,
            match_id,
            beatmap_id,
            score.user_id,
            score.accuracy as f64,
            score.mods.bits() as i16,
            score.score,
            score.stats.count50,
            score.stats.count100,
            score.stats.count300,
            score.stats.countgeki.unwrap_or(0),
            score.stats.countkatu.unwrap_or(0),
            score.stats.countmiss,
            score.max_combo.unwrap_or(0),
            detail.slot as i16,
            &detail.team,
            detail.pass,
        ).execute(&self.pool).await?;

        Ok(())
    }
}
