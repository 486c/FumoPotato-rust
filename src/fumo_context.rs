use crate::{stats::{BotMetrics, BotStats}, twitch_api::TwitchApi};
use std::io::Write;
use fumo_database::Database;
use osu_api::OsuApi;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use twilight_gateway::{
    stream, Config, ConfigBuilder, EventTypeFlags, Intents, Shard, ShardId,
};
use twilight_http::{client::InteractionClient, Client};
use twilight_model::id::{marker::ApplicationMarker, Id};
use twilight_standby::Standby;

use std::{collections::HashMap, env, fs::File, io::read_to_string, path::PathBuf, sync::Arc};

use eyre::Result;

pub static STATE_FILE: &str = ".fumo_state";

#[derive(Debug, Serialize, Deserialize)]
pub struct FumoContextState {
    pub osu_checker_last_cursor: Option<i64>,
}

impl Drop for FumoContextState {
    fn drop(&mut self) {
        let mut file = File::create(STATE_FILE)
            .expect("failed to open state file");

        let json_string = serde_json::to_string(&self)
            .expect("failed to serialize state");

        let _ = file.write_all(json_string.as_bytes());
    }
}

pub struct FumoContext {
    pub osu_api: OsuApi,
    pub twitch_api: TwitchApi,

    /// Checker list for all twitch users
    /// where key is twitch id
    /// and value is if streamer is online or not
    pub twitch_checker_list: Mutex<HashMap<i64, bool>>,

    pub db: Database,
    pub stats: BotMetrics,
    pub http: Arc<Client>,
    pub standby: Standby,

    application_id: Id<ApplicationMarker>,

    pub state: Mutex<FumoContextState>,
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
            env::var("TWITCH_SECRET")?.as_str(),
        )
        .await?;

        // Init osu api
        let osu_api = OsuApi::new(
            env::var("CLIENT_ID")?.parse()?,
            env::var("CLIENT_SECRET")?.as_str(),
            env::var("OSU_SESSION")?.as_str(),
            env::var("FALLBACK_API")?.as_str(),
            env::var("FALLBACK_API_KEY")?.as_str(),
            true,
        )
        .await?;

        let db = Database::init(env::var("DATABASE_URL")?.as_str()).await?;

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
                | Intents::MESSAGE_CONTENT,
        )
        .event_types(
            EventTypeFlags::INTERACTION_CREATE
                | EventTypeFlags::MESSAGE_CREATE
                | EventTypeFlags::MESSAGE_DELETE
                | EventTypeFlags::MESSAGE_UPDATE,
        )
        .build();

        let shards = stream::create_recommended(
            &http,
            config,
            |_shard_id: ShardId, builder: ConfigBuilder| builder.build(),
        )
        .await?
        .collect();

        let application_id =
            http.current_user().await?.model().await?.id.cast();

        let standby = Standby::new();

        let bot_metrics = BotStats::default();

        let stats = BotMetrics::new(
            osu_api.stats.counters.clone(),
            bot_metrics
        );

        // Trying to load state from file
        let state_path = PathBuf::from(STATE_FILE);

        let state = if state_path.exists() {
            let reader = File::open(state_path)?;
            let state_string = read_to_string(reader)?;

            let state: FumoContextState = serde_json::from_str(&state_string)?;

            state
        } else {
            FumoContextState {
                osu_checker_last_cursor: None,
            }
        };

        let ctx = FumoContext {
            osu_api,
            twitch_api,
            db,
            http,
            application_id,
            standby,
            stats,
            twitch_checker_list: Mutex::new(HashMap::new()),
            state: Mutex::new(state),
        };

        Ok((ctx, shards))
    }
}
