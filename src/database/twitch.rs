use eyre::Result;

use crate::database::Database;

use sqlx;

#[derive(sqlx::FromRow)]
pub struct TwitchStreamer {
    pub name: String,
    pub online: bool,
}

pub struct TwitchChannel {
    pub id: i64,
    pub name: String,
}

impl Database {
    /* TODO Rename everyting to something like TwitchTrackingStreamer & TwitchTrackingChannel or idk */
    pub async fn get_channels(&self, name: &str) -> Result<Vec<TwitchChannel>> {
        let channels = sqlx::query_as!(
            TwitchChannel,
            "SELECT * FROM twitch_channels WHERE name = $1",
            name
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

    pub async fn get_streamer(&self, name: &str) -> Option<TwitchStreamer> {
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

    pub async fn add_streamer(&self, name: &str) -> Result<TwitchStreamer> {
        let streamer = sqlx::query_as!(
            TwitchStreamer,
            "INSERT INTO twitch_streamers VALUES($1, false) 
            RETURNING name, online",
            name
        ).fetch_one(&self.pool).await?;

        Ok(streamer)
    }

    pub async fn add_tracking(&self, streamer: &TwitchStreamer, channel_id: i64) -> Result<()> {
        sqlx::query!(
            "INSERT INTO twitch_channels VALUES($2, $1)",
            &streamer.name, channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn remove_tracking(&self, name: &str, channel_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM twitch_channels WHERE 
            name = $1 and id = $2",
            name, channel_id
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn get_tracking(&self, name: &str, channel_id: i64) -> Option<TwitchChannel> {
        let track = sqlx::query_as!(
            TwitchChannel,
            "SELECT * FROM twitch_channels WHERE 
            name = $1 and id = $2",
            name, channel_id
        ).fetch_one(&self.pool).await;

        match track {
            Ok(s) => Some(s),
            Err(_) => None
        }
    }

    pub async fn toggle_online(&self, name: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE twitch_streamers SET online = NOT online WHERE name = $1",
            name
        ).execute(&self.pool).await?;

        Ok(())
    }
}
