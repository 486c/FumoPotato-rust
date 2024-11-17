use eyre::Result;

use crate::Database;

use sqlx;

#[derive(sqlx::FromRow, Debug)]
pub struct TwitchTrackedStreamer {
    pub twitch_id: i64,
    pub online: bool,
}

#[derive(Debug)]
pub struct TwitchChannel {
    pub twitch_id: i64,
    pub channel_id: i64,
}

impl Database {
    pub async fn get_channels_by_twitch_id(
        &self, 
        twitch_id: i64
    ) -> Result<Vec<TwitchChannel>> {
        let channels = sqlx::query_as!(
            TwitchChannel,
            "SELECT * FROM twitch_tracking WHERE twitch_id = $1",
            twitch_id
        ).fetch_all(&self.pool).await?;

        Ok(channels)
    }

    pub async fn get_channels_by_channel_id(
        &self, 
        id: i64
    ) -> Result<Vec<TwitchChannel>> {
        let channels = sqlx::query_as!(
            TwitchChannel,
            "SELECT * FROM twitch_tracking WHERE channel_id = $1",
            id
        ).fetch_all(&self.pool).await?;

        Ok(channels)
    }

    pub async fn get_streamers(
        &self
    ) -> Result<Vec<TwitchTrackedStreamer>> {
        let streamers = sqlx::query_as!(
            TwitchTrackedStreamer,
            "SELECT * FROM twitch_streamers"
        ).fetch_all(&self.pool)
        .await?;

        Ok(streamers)
    }

    pub async fn get_streamer(
        &self, 
        twitch_id: i64
    ) -> Option<TwitchTrackedStreamer> {
        let streamer = sqlx::query_as!(
            TwitchTrackedStreamer,
            "SELECT * FROM twitch_streamers WHERE twitch_id = $1",
            twitch_id 
        ).fetch_one(&self.pool)
        .await;

        match streamer {
            Ok(streamer) => Some(streamer),
            Err(_) => None,
        }
    }

    pub async fn add_streamer(
        &self, 
        twitch_id: i64
    ) -> Result<TwitchTrackedStreamer> {
        let streamer = sqlx::query_as!(
            TwitchTrackedStreamer,
            "INSERT INTO twitch_streamers(online, twitch_id) VALUES(false, $1) 
            RETURNING twitch_id, online",
            twitch_id
        ).fetch_one(&self.pool).await?;

        Ok(streamer)
    }

    pub async fn add_tracking(
        &self, 
        streamer: &TwitchTrackedStreamer, 
        channel_id: i64
    ) -> Result<()> {
        // Ensure that that discord channel exists
        self.add_discord_channel(channel_id).await?;

        sqlx::query!(
            "INSERT INTO twitch_tracking VALUES($2, $1)",
            streamer.twitch_id, channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn remove_tracking(
        &self, 
        twitch_id: i64, 
        channel_id: i64
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM twitch_tracking WHERE 
            twitch_id = $1 and channel_id = $2",
            twitch_id, channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn get_tracking(
        &self, 
        twitch_id: i64, 
        channel_id: i64
    ) -> Option<TwitchChannel> {
        let track = sqlx::query_as!(
            TwitchChannel,
            "SELECT * FROM twitch_tracking WHERE 
            twitch_id = $1 and channel_id = $2",
            twitch_id, channel_id
        ).fetch_one(&self.pool).await;

        match track {
            Ok(s) => Some(s),
            Err(_) => None
        }
    }

    pub async fn set_online_status(
        &self, 
        twitch_id: i64, 
        status: bool
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE twitch_streamers SET online = $2 WHERE twitch_id = $1",
            twitch_id, status
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn toggle_online(&self, twitch_id: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE twitch_streamers SET online = NOT online 
            WHERE twitch_id = $1",
            twitch_id
        ).execute(&self.pool).await?;

        Ok(())
    }
}
