use crate::osu_api::OsuApi;
use crate::twitch_api::TwitchApi;
use crate::database::Database;

pub struct FumoContext {
    pub osu_api: OsuApi,
    pub twitch_api: TwitchApi,
    pub db: Database,
}
