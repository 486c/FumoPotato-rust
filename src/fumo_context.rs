use crate::osu_api::OsuApi;
use crate::twitch_api::TwitchApi;
use twilight_gateway::cluster::Events;
use twilight_http::Client;
use twilight_gateway::{ EventTypeFlags, Intents, Cluster };

use std::env;
use std::sync::Arc;

pub struct FumoContext {
    pub osu_api: OsuApi,
    pub twitch_api: TwitchApi,
    pub http: Arc<Client>,
    pub cluster: Cluster,
}

impl FumoContext {
    pub async fn new(token: &str) -> (FumoContext, Events)  {
        // Init twitch api
        let twitch_api = TwitchApi::init(
            env::var("TWITCH_TOKEN").unwrap().as_str(),
            env::var("TWITCH_CLIENT_ID").unwrap().as_str()
        ).await.unwrap();

        // Init osu api
        let osu_api = OsuApi::init(
            env::var("CLIENT_ID").unwrap().parse().unwrap(),
            env::var("CLIENT_SECRET").unwrap().as_str(),
            env::var("FALLBACK_API").unwrap().as_str(),
            true
        ).await.unwrap();

        let http = Client::builder()
            .token(token.to_owned())
            .remember_invalid_token(false)
            .build();

        let http = Arc::new(http);

        let (cluster, events) = Cluster::builder(token.to_owned(), Intents::GUILD_MESSAGES)
            .http_client(Arc::clone(&http))
            .event_types(
                EventTypeFlags::INTERACTION_CREATE
                | EventTypeFlags::MESSAGE_CREATE
                | EventTypeFlags::MESSAGE_DELETE
                | EventTypeFlags::MESSAGE_UPDATE
            )
            .build()
            .await.unwrap();

        let ctx = FumoContext {
            osu_api,
            twitch_api,
            http,
            cluster,
        };

        (ctx, events)
    }
}
