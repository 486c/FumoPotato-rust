mod datetime;
mod models;

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{ Client, StatusCode, Method, Response };

use self::models::{OauthResponse, OsuBeatmap, OsuLeaderboard};

use anyhow::Result;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use tokio::sync::oneshot::{ channel, Receiver, Sender };

#[derive(Debug)]
pub struct OsuApi {
    inner: Arc<OsuToken>,
    fallback_url: String,
    loop_drop_tx: Option<Sender<()>>,
}

impl Drop for OsuApi {
    fn drop(&mut self) {
        if let Some(tx) = self.loop_drop_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Debug)]
pub struct OsuToken {
    client: Client,

    client_id: i32,
    secret: String,
    token: RwLock<String>,
}

impl OsuToken {
    async fn request_oauth(&self) -> Result<OauthResponse> {
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

        Ok(json_data)
    }
}

impl OsuApi {

    async fn make_request(&self, link: &str, method: Method) -> Result<Response> {
        let token = self.inner.token.read().await;

        let r = &self.inner.client;
        let r = match method {
            Method::GET => r.get(link),
            Method::POST => r.post(link),
            _ => unimplemented!(),
        };

        let r = r
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {}", token));

        Ok(r.send().await?)
    }

    pub async fn get_beatmap(&self, bid: i32) -> Option<OsuBeatmap> {
        let link = format!("https://osu.ppy.sh/api/v2/beatmaps/{}", bid);
        let r = self.make_request(&link, Method::GET).await.unwrap();
        
        if r.status() != StatusCode::OK {
            return None;
        }
        
        let b = r.json::<OsuBeatmap>().await.unwrap();

        Some(b)
    }

    // This method works only if FALLBACK_API variable
    // is set.
    pub async fn get_countryleaderboard(&self, bid: i32) -> Option<OsuLeaderboard> {
        let link = format!(
            "{}/leaderboard/leaderboard?beatmap={}&type=country",
            self.fallback_url,
            bid
        );

        let r = self.make_request(&link, Method::GET).await.unwrap();

        if r.status() != StatusCode::OK {
            return None;
        }

        let b = r.json::<OsuLeaderboard>().await.unwrap();

        Some(b)
    }
}

impl OsuApi {
    pub async fn init(
        client_id: i32,
        secret: &str,
        fallback_url: &str,
        run_loop: bool
    ) -> Result<OsuApi> {
        let inner = Arc::new(OsuToken {
            client: Client::new(),
            client_id,
            secret: secret.to_owned(),
            token: Default::default(),
        });

        let response = inner.request_oauth().await.unwrap();

        let mut token = inner.token.write().await;
        *token = response.access_token;
        drop(token);

        let (tx, rx) = channel::<()>();
    
        if run_loop {
            OsuApi::update_token(
                Arc::clone(&inner), 
                response.expires_in as u64,
                rx
            ).await;
        }

        let api = OsuApi {
            loop_drop_tx: Some(tx),
            inner,
            fallback_url: fallback_url.to_owned(),
        };

        Ok(api)
    }

    async fn update_token(osu: Arc<OsuToken>, expire: u64, rx: Receiver<()>) {
        tokio::spawn(async move {
            OsuApi::token_loop(Arc::clone(&osu), expire, rx).await;
            println!("osu!api token loop is closed!");
        });
    }

    async fn token_loop(osu: Arc<OsuToken>, mut expire: u64, mut rx: Receiver<()>) {
        loop {
            expire /= 2;
            println!("Token update scheduled in {} seconds", expire);
            tokio::select!{
                _ = tokio::time::sleep(Duration::from_secs(expire)) => {}
                _ = &mut rx => {
                    return;
                }
            }

            let response = osu.request_oauth().await.unwrap();

            let mut token = osu.token.write().await;
            *token = response.access_token;

            expire = response.expires_in as u64;
            println!("Successfully updated osu! token!");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::osu_api::{
        *,
        models::*
    };

    use std::env;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_get_beatmap() {
        dotenv().unwrap();

        let api = OsuApi::init(
            env::var("CLIENT_ID").unwrap().parse().unwrap(),
            env::var("CLIENT_SECRET").unwrap().as_str(),
            env::var("FALLBACK_API").unwrap().as_str(),
            false
        ).await.unwrap();

        let mut op = api.get_beatmap(3153603).await;

        assert!(!op.is_none());

        let b = op.unwrap();
        assert_eq!(b.id, 3153603);

        op = api.get_beatmap(12).await;
        assert!(op.is_none());

        let op = api.get_beatmap(1173889).await.unwrap();
        assert_eq!(op.ranked, RankStatus::Loved);

        let op = api.get_beatmap(3833489).await.unwrap();
        assert_eq!(op.ranked, RankStatus::Graveyard);

        let op = api.get_beatmap(3818011).await.unwrap();
        assert_eq!(op.ranked, RankStatus::Ranked);
    }
    
    #[tokio::test]
    async fn test_makerequest() {
        dotenv().unwrap();

        let api = OsuApi::init(
            env::var("CLIENT_ID").unwrap().parse().unwrap(),
            env::var("CLIENT_SECRET").unwrap().as_str(),
            env::var("FALLBACK_API").unwrap().as_str(),
            false
        ).await.unwrap();

        api.make_request("https://google.com", Method::GET).await.unwrap();
    }

    #[test]
    fn mods_test() {
        let mods = OsuMods::NOMOD;
        assert_eq!(mods.to_string(), "NM");

        let mods = OsuMods::NOMOD | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HD");

        let mods = OsuMods::HARDROCK | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HDHR");

        let mods = OsuMods::DOUBLETIME | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HDDT");

        let mods = OsuMods::NIGHTCORE;
        assert_eq!(mods.to_string(), "NC");

        let mods = OsuMods::NIGHTCORE | OsuMods::HIDDEN;
        assert_eq!(mods.to_string(), "HDNC");

        let mods = OsuMods::NIGHTCORE | OsuMods::HIDDEN | OsuMods::HARDROCK;
        assert_eq!(mods.to_string(), "HDNCHR");
    }
}
