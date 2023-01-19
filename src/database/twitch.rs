use eyre::Result;

use crate::database::Database;

use sqlx;

#[derive(sqlx::FromRow, Debug)]
pub struct TwitchStreamer {
    pub name: String,
    pub id: i64,
    pub online: bool,
}

#[derive(Debug)]
pub struct TwitchChannel {
    pub id: i64,
    pub channel_id: i64,
}

impl Database {
    /* TODO Rename everyting to something like TwitchTrackingStreamer & TwitchTrackingChannel or idk */
    pub async fn get_channels(&self, id: i64) -> Result<Vec<TwitchChannel>> {
        let channels = sqlx::query_as!(
            TwitchChannel,
            "SELECT * FROM twitch_tracking WHERE id = $1",
            id
        ).fetch_all(&self.pool).await?;

        Ok(channels)
    }

    pub async fn get_streamers(&self) -> Result<Vec<TwitchStreamer>> {
        let streamers = sqlx::query_as!(
            TwitchStreamer,
            "SELECT * FROM twitch_streamers"
        ).fetch_all(&self.pool)
        .await?;

        Ok(streamers)
    }

    pub async fn get_streamer(&self, id: i64) -> Option<TwitchStreamer> {
        let streamer = sqlx::query_as!(
            TwitchStreamer,
            "SELECT * FROM twitch_streamers WHERE id = $1",
            id 
        ).fetch_one(&self.pool)
        .await;

        match streamer {
            Ok(streamer) => Some(streamer),
            Err(_) => None,
        }
    }

    pub async fn get_streamer_by_name(&self, name: &str) -> Option<TwitchStreamer> {
        let streamer = sqlx::query_as!(
            TwitchStreamer,
            "SELECT * FROM twitch_streamers WHERE name = $1",
            name
        ).fetch_one(&self.pool)
        .await;

        match streamer {
            Ok(streamer) => Some(streamer),
            Err(_) => None,
        }
    }

    pub async fn add_streamer(&self, id: i64, name: &str) -> Result<TwitchStreamer> {
        let streamer = sqlx::query_as!(
            TwitchStreamer,
            "INSERT INTO twitch_streamers(name, online, id) VALUES($1, false, $2) 
            RETURNING id, name, online",
            name, id
        ).fetch_one(&self.pool).await?;

        Ok(streamer)
    }

    pub async fn add_tracking(&self, streamer: &TwitchStreamer, channel_id: i64) -> Result<()> {
        sqlx::query!(
            "INSERT INTO twitch_tracking VALUES($2, $1)",
            streamer.id, channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn remove_tracking(&self, id: i64, channel_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM twitch_tracking WHERE 
            id = $1 and channel_id = $2",
            id, channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn get_tracking(&self, id: i64, channel_id: i64) -> Option<TwitchChannel> {
        let track = sqlx::query_as!(
            TwitchChannel,
            "SELECT * FROM twitch_tracking WHERE 
            id = $1 and channel_id = $2",
            id, channel_id
        ).fetch_one(&self.pool).await;

        match track {
            Ok(s) => Some(s),
            Err(_) => None
        }
    }

    pub async fn toggle_online(&self, id: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE twitch_streamers SET online = NOT online WHERE id = $1",
            id
        ).execute(&self.pool).await?;

        Ok(())
    }
}
