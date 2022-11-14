use crate::osu_api::OsuApi;
use crate::twitch_api::TwitchApi;
use twilight_gateway::cluster::Events;
use twilight_http::Client;
use twilight_http::client::InteractionClient;
use twilight_gateway::{ EventTypeFlags, Intents, Cluster };
use twilight_model::id::{
    Id, 
    marker::ApplicationMarker
};
use twilight_standby::Standby;

use std::env;
use std::sync::Arc;

pub struct FumoContext {
    pub osu_api: OsuApi,
    pub twitch_api: TwitchApi,
    pub http: Arc<Client>,
    pub cluster: Cluster,
    pub standby: Standby,

    application_id: Id<ApplicationMarker>,
}

impl FumoContext {
    pub fn interaction(&self) -> InteractionClient<'_> {
        self.http.interaction(self.application_id)
    }
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

        let (cluster, events) = Cluster::builder(
            token.to_owned(), 
            Intents::all(),
        )
            .http_client(Arc::clone(&http))
            .event_types(
                EventTypeFlags::INTERACTION_CREATE
                | EventTypeFlags::MESSAGE_CREATE
                | EventTypeFlags::MESSAGE_DELETE
                | EventTypeFlags::MESSAGE_UPDATE
                | EventTypeFlags::SHARD_PAYLOAD
                )
            .build()
            .await.unwrap();


        let application_id = http.current_user()
            .exec()
            .await.unwrap()
            .model()
            .await.unwrap()
            .id.cast();

        let standby = Standby::new();

        let ctx = FumoContext {
            osu_api,
            twitch_api,
            http,
            cluster,
            application_id,
            standby,
        };

        (ctx, events)
    }
}
