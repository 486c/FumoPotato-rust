macro_rules! discord_id {
    ($cmd:ident) => {
        match $cmd.user_id() {
            Some(id) => id,
            None => 
                return Err(eyre::Report::msg("No discord user id"))
        }
    };
}

macro_rules! osu_user {
    ($ctx:ident, $cmd:ident) => {{
        let discord_id = discord_id!($cmd);

        $ctx.db.get_osu_db_user(discord_id.get() as i64)
            .await?
    }};
}

pub mod country_leaderboard;
pub mod twitch;
pub mod attributes;
pub mod osu;
pub mod osu_tracking;
