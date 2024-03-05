macro_rules! discord_id {
    ($cmd:ident) => {
        match $cmd.user_id() {
            Some(id) => id,
            None => 
                return Err(eyre::Report::msg("No discord user id"))
        }
    };
}

/// Gets [`OsuDbUser`] from discord id
macro_rules! osu_user {
    ($ctx:ident, $cmd:ident) => {{
        let discord_id = discord_id!($cmd);

        $ctx.db.get_osu_db_user(discord_id.get() as i64)
            .await?
    }};
}

/// Inserts new tracking user while preserving all
/// database relations
macro_rules! add_osu_tracking_user{
    ($ctx:ident, $osu_id:expr, $discord_channel_id:expr) => {{
        // Insert discord channel
        $ctx.db.add_discord_channel($discord_channel_id).await?;

        // Insert new tracked user
        $ctx.db.add_tracked_osu_user(
            $osu_id
        ).await?;
        
        // Insert new tracking
        $ctx.db.add_osu_tracking(
            $discord_channel_id,
            $osu_id
        ).await?;
    }};
}

pub mod country_leaderboard;
pub mod twitch;
pub mod attributes;
pub mod osu;
pub mod osu_tracking;
