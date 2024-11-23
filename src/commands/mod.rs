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
    ($ctx:ident, $osu:expr, $discord_channel_id:expr) => {{
        // Insert discord channel
        $ctx.db.add_discord_channel($discord_channel_id).await?;

        // Insert osu player to kinda cache username
        $ctx.db.add_osu_player($osu.id, &$osu.username).await?;

        // Insert new tracked user
        $ctx.db.add_tracked_osu_user(
            $osu.id
        ).await?;
        
        // Insert new tracking
        $ctx.db.add_osu_tracking(
            $discord_channel_id,
            $osu.id
        ).await?;
    }};
}

macro_rules! component_stream {
    ($ctx:ident, $msg:expr) => {{
        let stream = $ctx.standby
            .wait_for_component_stream($msg.id, |_: &Interaction| {
                true
            }) 
        .map(|event| {
            let Interaction {
                channel,
                data,
                guild_id,
                kind,
                id,
                token,
                ..
            } = event;

            if let Some(
                InteractionData::MessageComponent(data)
                ) = data {
                InteractionComponent {
                    channel,
                    data: Some(data),
                    kind,
                    id,
                    token,
                    guild_id
                } 
            } else {
                InteractionComponent {
                    channel,
                    data: None,
                    kind,
                    id,
                    token,
                    guild_id
                } 
            }
        })
        .timeout(Duration::from_secs(20));        


        stream
    }}
}

pub mod country_leaderboard;
pub mod twitch;
pub mod attributes;
pub mod osu;
pub mod osu_tracking;
pub mod multiplayer;
