use crate::osu_api::OsuApi;
use crate::twitch_api::TwitchApi;
use crate::database::Database;
use crate::stats::BotStats;

use twilight_http::Client;
use twilight_http::client::InteractionClient;
use twilight_gateway::{ EventTypeFlags, Intents,  Config, stream, ShardId, ConfigBuilder, Shard };
use twilight_model::id::{
    Id, 
    marker::ApplicationMarker
};
use twilight_standby::Standby;

use std::env;
use std::sync::Arc;

use eyre::Result;

pub struct FumoContext {
    pub osu_api: OsuApi,
    pub twitch_api: TwitchApi,
    pub db: Database,
    pub stats: BotStats,
    pub http: Arc<Client>,
    pub standby: Standby,

    application_id: Id<ApplicationMarker>,
}

impl FumoContext {
    pub fn interaction(&self) -> InteractionClient<'_> {
        self.http.interaction(self.application_id)
    }
}

impl FumoContext {
    pub async fn new(token: &str) -> Result<(FumoContext, Vec<Shard>)> {
        // Init twitch api
        let twitch_api = TwitchApi::new(
            env::var("TWITCH_CLIENT_ID")?.as_str(),
            env::var("TWITCH_SECRET")?.as_str()
        ).await?;

        // Init osu api
        let osu_api = OsuApi::new(
            env::var("CLIENT_ID")?.parse()?,
            env::var("CLIENT_SECRET")?.as_str(),
            env::var("OSU_SESSION")?.as_str(),
            env::var("FALLBACK_API")?.as_str(),
            true
        ).await?;

        let db = Database::init(
            env::var("DATABASE_URL")?.as_str(),
        ).await?;

        let http = Client::builder()
            .token(token.to_owned())
            .remember_invalid_token(false)
            .build();

        let http = Arc::new(http);

        let config = Config::builder(
            token.to_owned(),
            Intents::MESSAGE_CONTENT
            | Intents::DIRECT_MESSAGES
            | Intents::DIRECT_MESSAGE_REACTIONS
            | Intents::MESSAGE_CONTENT
        )
        .event_types(
            EventTypeFlags::INTERACTION_CREATE
            | EventTypeFlags::MESSAGE_CREATE
            | EventTypeFlags::MESSAGE_DELETE
            | EventTypeFlags::MESSAGE_UPDATE
        )
        .build();

        let shards = stream::create_recommended(
            &http,
            config,
            |_shard_id: ShardId, builder: ConfigBuilder| builder.build()
        )
        .await?
        .collect();

        let application_id = http.current_user()
            .await?
            .model()
            .await?
            .id.cast();

        let standby = Standby::new();

        let stats = BotStats::new(osu_api.stats.counters.clone());

        let ctx = FumoContext {
            osu_api,
            twitch_api,
            db,
            http,
            application_id,
            standby,
            stats,
        };

        Ok((ctx, shards))
    }
}
