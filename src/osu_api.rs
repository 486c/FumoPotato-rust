use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use serenity::async_trait;

use crate::datetime::deserialize_local_datetime;

use chrono::prelude::*;

use crate::config::BotConfig;
use anyhow::Result;

#[derive(Deserialize, Debug)]
struct OauthResponse {
    token_type: String,
    expires_in: i32,
    access_token: String,
}

#[derive(Deserialize, Debug)]
pub struct OsuBeatmapsetCompact {
    title: String,
    artist: String,
    creator: String,

}

#[derive(Deserialize, Debug)]
pub struct OsuBeatmap {
    pub beatmapset_id: i32,
    pub id: i32,
    pub mode: String,

    pub version: String,

    pub beatmapset: OsuBeatmapsetCompact,
}

impl OsuBeatmap {
    pub fn metadata(&self) -> String {
        format!("{} - {} [{}]", self.beatmapset.artist, self.beatmapset.title, self.version)
    }
}

#[derive(Deserialize, Debug)]
pub struct OsuScore {
    pub id: i64,
    pub best_id: i64,
    pub user_id: i64,
    pub accuracy: f32,
    //mods
    pub score: i64,
    pub max_combo: i32,
    pub perfect: bool,
    pub passed: bool,
    pub pp: Option<f32>,
    pub rank: String,

    #[serde(deserialize_with = "deserialize_local_datetime")]
    pub created_at: DateTime<Utc>,

    pub mode: String,
    pub mode_int: i16,
    pub replay: bool,
    pub user: OsuUserCompact,
}

#[derive(Deserialize, Debug)]
pub struct OsuLeaderboard {
    pub scores: Vec<OsuScore>,
}

#[derive(Deserialize, Debug)]
pub struct OsuUserCompact {
    pub avatar_url: String,
    pub country_code: String,
    pub default_group: String,
    pub id: i64,
    pub is_active: bool,
    pub is_bot: bool,
    pub is_deleted: bool,
    pub is_online: bool,
    pub is_supporter: bool,
    pub pm_friends_only: bool,
    pub username: String,
    // last_visit & profile_colour skipped
}

#[derive(Debug)]
pub struct OsuApi {
    client: Client,
    client_id: i32,
    secret: String,

    token: String,
}

#[async_trait]
pub trait Body {
    async fn update_token(&mut self) -> Result<()>;
    async fn get_beatmap(&self, bid: i32) -> Option<OsuBeatmap>;
    async fn get_countryleaderboard(&self, bid: i32) -> Option<OsuLeaderboard>;
}

#[async_trait]
impl Body for OsuApi {
    async fn update_token(&mut self) -> Result<()> {
        let data = format!(
            r#"{{
            "client_id":"{}",
            "client_secret":"{}",
            "grant_type":"client_credentials",
            "scope":"public" 
        }}"#,
            &self.client_id, &self.secret
        );

        let r = self
            .client
            .post("https://osu.ppy.sh/oauth/token")
            .body(data.into_bytes())
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .unwrap();

        let json_data = r.json::<OauthResponse>().await?;
        self.token = json_data.access_token;

        Ok(())
    }

    async fn get_beatmap(&self, bid: i32) -> Option<OsuBeatmap> {
        let link = format!("https://osu.ppy.sh/api/v2/beatmaps/{}", bid);

        let r = self
            .client
            .get(link)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await
            .unwrap();

        if r.status() != StatusCode::OK {
            return None;
        }
        
        let b = r.json::<OsuBeatmap>().await.unwrap();

        Some(b)
    }

    // This method works only if FALLBACK_API variable
    // is set.
    async fn get_countryleaderboard(&self, bid: i32) -> Option<OsuLeaderboard> {
        let cfg = match BotConfig::get_res() {
            Some(c) => c,
            None => return None,
        };

        let link = format!(
            "{}/leaderboard/leaderboard?beatmap={}&type=country",
            cfg.fallback_api,
            bid
        );

        let r = self
            .client
            .get(link)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await
            .unwrap();

        if r.status() != StatusCode::OK {
            return None;
        }

        let b = r.json::<OsuLeaderboard>().await.unwrap();

        Some(b)
    }
}

impl OsuApi {
    pub async fn init(client_id: i32, secret: &str) -> Result<OsuApi> {
        let mut api: OsuApi = OsuApi {
            client: Client::new(),
            client_id,
            secret: secret.to_string(),

            token: Default::default(),
        };

        api.update_token().await?;

        Ok(api)
    }
}

#[cfg(test)]
mod tests {
    use crate::osu_api::*;

    use std::env;
    use dotenv::dotenv;


    #[tokio::test]
    async fn test_something() {
        dotenv().unwrap();

        let api = OsuApi::init(
            env::var("CLIENT_ID").unwrap().parse().unwrap(),
            env::var("CLIENT_SECRET").unwrap().as_str(),
        ).await.unwrap();

        let mut op = api.get_beatmap(3153603).await;

        assert!(!op.is_none());

        let b = op.unwrap();
        assert_eq!(b.id, 3153603);

        op = api.get_beatmap(12).await;
        assert!(op.is_none());
    }
}
