mod datetime;
mod metrics;

pub mod models;
pub mod error;

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, COOKIE, USER_AGENT};
use reqwest::{ Client, StatusCode, Method, Response };

use self::models::osu_leaderboard::OsuLeaderboardLazer;
use self::models::{ 
    OauthResponse, OsuBeatmap, OsuLeaderboard, 
    ApiError, UserId, OsuGameMode, OsuUserExtended, GetUserScores, OsuScore, GetRanking, Rankings, RankingKind 
};

use std::fmt::Write;

use self::metrics::Metrics;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{ RwLock, oneshot::{ channel, Receiver, Sender } };

use crate::osu_api::error::OsuApiError;
use crate::osu_api::models::OsuUserStatistics;
use serde::de::DeserializeOwned;

static OSU_BASE: &str = "https://osu.ppy.sh";
static OSU_API_BASE: &str = "https://osu.ppy.sh/api/v2";

type ApiResult<T> = Result<T, OsuApiError>;

pub enum ApiKind {
    General,
    Hidden,
}

#[derive(Debug)]
pub struct OsuApi {
    inner: Arc<OsuToken>,
    fallback_url: String,
    loop_drop_tx: Option<Sender<()>>,
    osu_session: String,
    pub stats: Metrics
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
    async fn request_oauth(&self) -> ApiResult<OauthResponse> {
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
    async fn make_request<T: DeserializeOwned>(
        &self, 
        link: &str, 
        method: Method,
        api_kind: ApiKind,
    ) -> ApiResult<T> {
        let token = self.inner.token.read().await;

        let r = &self.inner.client;
        let r = match method {
            Method::GET => r.get(link),
            Method::POST => r.post(link),
            _ => unimplemented!(),
        };

        let req = match api_kind {
            ApiKind::General => {
                r
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, format!("Bearer {token}"))
            },
            ApiKind::Hidden => {
                r
                .header(USER_AGENT, "fumo_potato")
                .header(COOKIE, format!("osu_session={}", self.osu_session))
            },
        };

        let resp = req.send().await?;

        self.handle_error(resp).await
    }

    async fn handle_error<T: DeserializeOwned>(
        &self, 
        r: Response
    ) -> ApiResult<T> {
        match r.status() {
            StatusCode::OK => {
                //TODO move this nesting mess outta here
                let bytes = r.bytes().await?;
                return serde_json::from_slice::<T>(&bytes)
                    .map_err(|s| {
                        // TODO wrap serde error
                        // for more informative response
                        OsuApiError::Parsing {
                            source: s,
                            body: std::str::from_utf8(&bytes).unwrap().to_owned(),
                        }
                    });
            }
            StatusCode::NOT_FOUND => return Err(
                OsuApiError::NotFound {
                    url: r.url().to_string(),
                }
            ),
            StatusCode::TOO_MANY_REQUESTS => 
                return Err(OsuApiError::TooManyRequests),
            StatusCode::UNPROCESSABLE_ENTITY => {
                let bytes = r.bytes().await?;

                return Err(OsuApiError::UnprocessableEntity{
                    body: std::str::from_utf8(&bytes).unwrap().to_owned(),
                })
            },
            _ => (),
        };

        let bytes = r.bytes().await?;
        let parsed: ApiError = match serde_json::from_slice(
            &bytes
        ) {
            Ok(v) => v,
            Err(e) => { 
                if bytes.len() <= 1 {
                    return Err(OsuApiError::EmptyBody)
                }

                return Err(OsuApiError::Parsing {
                    source: e,
                    body: std::str::from_utf8(&bytes).unwrap().to_owned(),
                })
            }
        };

        Err(OsuApiError::ApiError {
            source: parsed,
        })
    }

    pub async fn get_user_scores(
        &self,
        user_scores: GetUserScores
    ) -> ApiResult<Vec<OsuScore>> {
        let mut link = format!(
            "{OSU_API_BASE}/users/{}/scores/{}?",
            user_scores.user_id,
            user_scores.kind
        );

        // TODO think of better ways of handling query arguments
        // TODO handle pagination 

        if let Some(limit) = user_scores.limit {
            link.push_str(&format!("limit={limit}&"))
        }

        if let Some(fails) = user_scores.include_fails {
            let fails = match fails {
                true => "1",
                false => "0",
            };

            link.push_str(&format!("include_fails={fails}&"))
        }

        let r = self.make_request(
            &link[..link.len()-1], 
            Method::GET,
            ApiKind::General,
        ).await?;

        Ok(r)
    }

    pub async fn get_user(
        &self,
        user_id: UserId,
        mode: Option<OsuGameMode>
    ) -> ApiResult<Option<OsuUserExtended>> {
        let mut link = OSU_API_BASE.to_owned();

        // TODO ?key=
        link.push_str(&format!("/users/{user_id}"));

        if let Some(mode) = mode {
            link.push_str(&format!("/mode/{mode}"))
        }

        let r: ApiResult<OsuUserExtended> = self.make_request(
            &link, 
            Method::GET, 
            ApiKind::General
        ).await;

        match r {
            Ok(v) => {
                Ok(Some(v))
            },
            Err(e) => {
                match e {
                    OsuApiError::NotFound { .. } => Ok(None),
                    _ => Err(e)
                }
            },
        }
    }
    
    pub async fn get_beatmap(
        &self, 
        bid: i32
    ) -> ApiResult<OsuBeatmap> {
        let link = format!(
            "{OSU_API_BASE}/beatmaps/{bid}"
        );

        let r = self.make_request(
            &link, 
            Method::GET, 
            ApiKind::General
        ).await?;

        self.stats.beatmap.inc();

        Ok(r)
    }

    pub async fn get_rankings(
        &self,
        ranking: &GetRanking,
        amount: usize,
    ) -> ApiResult<Rankings> {

        let mut link = String::with_capacity(50);
        let mut buffer: Vec<OsuUserStatistics> = 
            Vec::with_capacity(amount);

        let pages: usize = (
            (amount as f32 / 50.0).ceil() as usize
        ).max(1);

        let pages_offset = ranking.page.unwrap_or(0) as usize;

        let pages_range = if pages_offset != 0 {
            pages_offset..=pages_offset+pages
        } else {
            1..=pages
        };

        for page in pages_range {
            link.clear();
            let _ = write!(
                link, 
                "{OSU_API_BASE}/rankings/{}/{}?filter={}&cursor[page]={}",
                ranking.mode, ranking.kind, ranking.filter, page
            );

            if let (RankingKind::Performance, Some(country)) = (&ranking.kind, &ranking.country) {
                let _ = write!(
                    link, 
                    "&country={}", 
                    &country
                );
            };
            
            let res: Rankings = self.make_request(
                &link,
                Method::GET,
                ApiKind::General
            ).await?;

            let amount_to_append = (
                amount - buffer.len()
            ).min(res.ranking.len());

            buffer.extend_from_slice(&res.ranking[0..amount_to_append])
        }

        Ok(Rankings {
            ranking: buffer
        })
    }

    pub async fn get_leaderboard_hidden(
        &self,
        bid: i32,
        country: bool,
    ) -> ApiResult<OsuLeaderboardLazer> {
        let mut link = format!(
            "{OSU_BASE}/beatmaps/{bid}/scores?"
        );

        if country {
            link.push_str("type=country")
        }

        self.make_request(
            &link,
            Method::GET,
            ApiKind::Hidden
        ).await
    }

    // This method works only if FALLBACK_API variable
    // is set.
    pub async fn get_countryleaderboard_fallback(
        &self, 
        bid: i32
    ) -> ApiResult<OsuLeaderboard> {
        let link = format!(
            "{}/leaderboard/leaderboard?beatmap={}&type=country",
            self.fallback_url,
            bid
        );
        
        // FIXME Temporary solution since seneaL's api is BS
        // at the moment
        let mut retries = 0;
        while retries <= 5 {
            let resp = self.make_request(
                &link, 
                Method::GET, 
                ApiKind::General
            ).await;

            self.stats.country_leaderboard.inc();

            match resp {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if let OsuApiError::EmptyBody = e {
                        retries += 1;
                        continue
                    } else {
                        return Err(e)
                    }
                },
            }
        }

        Err(OsuApiError::ExceededMaxRetries)
    }
}

impl OsuApi {
    pub async fn new(
        client_id: i32,
        secret: &str,
        osu_session: &str,
        fallback_url: &str,
        run_loop: bool
    ) -> ApiResult<OsuApi> {
        let inner = Arc::new(OsuToken {
            client: Client::new(),
            client_id,
            secret: secret.to_owned(),
            token: Default::default(),
        });

        let response = inner.request_oauth().await?;

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

        let stats = Metrics::new();

        let api = OsuApi {
            loop_drop_tx: Some(tx),
            inner,
            fallback_url: fallback_url.to_owned(),
            osu_session: osu_session.to_owned(),
            stats,
        };

        Ok(api)
    }

    async fn update_token(
        osu: Arc<OsuToken>,
        expire: u64, 
        rx: Receiver<()>
    ) {
        tokio::spawn(async move {
            OsuApi::token_loop(Arc::clone(&osu), expire, rx).await;
            println!("osu!api token loop is closed!");
        });
    }

    async fn token_loop(
        osu: Arc<OsuToken>, 
        mut expire: u64, 
        mut rx: Receiver<()>
    ) {
        loop {
            expire /= 2;
            println!("osu! token update scheduled in {expire} seconds");
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
    use std::str::FromStr;
    use crate::osu_api::{
        *,
        models::*
    };

    use std::env;
    use chrono::{NaiveDate, NaiveDateTime};
    use dotenv::dotenv;
    use async_once_cell::OnceCell;

    static API_INSTANCE: OnceCell<OsuApi> = OnceCell::new();

    async fn get_api() -> &'static OsuApi {
        dotenv().unwrap();

        API_INSTANCE.get_or_init(async {
            OsuApi::new(
                env::var("CLIENT_ID").unwrap().parse().unwrap(),
                env::var("CLIENT_SECRET").unwrap().as_str(),
                env::var("OSU_SESSION").unwrap().as_str(),
                env::var("FALLBACK_API").unwrap().as_str(),
                false
            )
            .await
            .expect("Failed to initialize osu api")
        }).await
    }

    #[tokio::test]
    async fn test_get_leaderboard_hidden() {
        let api = get_api().await;

        let leaderboard = api.get_leaderboard_hidden(
            1804553, 
            false
        ).await.unwrap();

        assert!(leaderboard.scores.len() == 50);

        let leaderboard = api.get_leaderboard_hidden(
            1804553, 
            true
        ).await.unwrap();

        assert!(leaderboard.scores.len() > 2);
    }
    
    #[tokio::test]
    async fn test_get_scores() {
        let api = get_api().await;

        let req = GetUserScores::new(
            6892711, 
            ScoresType::Best
        )
        .limit(10);

        let scores = api.get_user_scores(req).await.unwrap();
        assert_eq!(scores.len(), 10);

        let req = GetUserScores::new(
            7562902, 
            ScoresType::Recent
        )
        .limit(50)
        .include_fails(true);

        api.get_user_scores(req).await.unwrap();
    }

    #[tokio::test]
    async fn test_api_timezone() {
        let api = get_api().await;

        let req = GetUserScores::new(
            15555817, 
            ScoresType::Best
        )
        .limit(100);

        let scores = api.get_user_scores(req).await.unwrap();

        let score = scores.iter().find(|x| {
            if let Some(beatmap) = &x.beatmap {
                return beatmap.id == 1402392;
            };

            false
        });

        assert_eq!(score.is_some(), true);

        let score = score.unwrap();

        let dt: NaiveDateTime = NaiveDate::from_ymd_opt(2024, 3, 9)
            .unwrap()
            .and_hms_opt(20, 12, 0).unwrap();

        let score_dt = score.created_at.naive_utc();

        dbg!(dt);
        dbg!(score_dt);

        assert!(score_dt > dt);

        dbg!(score.created_at);
    }

    #[tokio::test]
    async fn test_get_user() {
        let api = get_api().await;

        let user = api.get_user(
            UserId::Id(6892711),
            None
        ).await.unwrap().unwrap();

        assert_eq!(user.id, 6892711);
        assert_eq!(user.username, "LoPij");
        assert_eq!(user.country_code, "BY");

        let user = api.get_user(
            UserId::Username("DaHuJka".to_owned()),
            None
        ).await.unwrap().unwrap();

        assert_eq!(user.id, 6830745);
        assert_eq!(user.username, "DaHuJka");
        assert_eq!(user.country_code, "RU");

        let user = api.get_user(
            UserId::Id(34785329384),
            None
        ).await.unwrap();

        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_get_beatmap() {
        let api = get_api().await;

        let mut op = api.get_beatmap(3153603).await;

        assert!(op.is_ok());

        let b = op.unwrap();
        assert_eq!(b.id, 3153603);

        op = api.get_beatmap(12).await;
        assert!(op.is_err());

        let op = api.get_beatmap(1173889).await.unwrap();
        assert_eq!(op.ranked, RankStatus::Loved);

        let op = api.get_beatmap(3833489).await.unwrap();
        assert_eq!(op.ranked, RankStatus::Graveyard);

        let op = api.get_beatmap(3818011).await.unwrap();
        assert_eq!(op.ranked, RankStatus::Ranked);
    }
    
    #[tokio::test]
    #[should_panic]
    async fn test_notfound_error() {
        let api = get_api().await;

        let link = "https://osu.ppy.sh/apii/v2/beaaps/";
        let _: OsuBeatmap = api.make_request(
            link, Method::GET, ApiKind::General
        )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_rankings_country() {
        let api = get_api().await;

        let req = GetRanking {
            mode: OsuGameMode::Osu,
            kind: RankingKind::Performance,
            filter: RankingFilter::All,
            country: Some("BY".to_owned()),
            page: None
        };

        let res = api.get_rankings(
            &req,
            50
        ).await.unwrap();

        assert_eq!(50, res.ranking.len());
        assert_eq!("BY", res.ranking[0].user.country_code);
        assert_eq!("BY", res.ranking[20].user.country_code);
        assert_eq!("BY", res.ranking[49].user.country_code);
    }

    #[tokio::test]
    async fn test_get_rankings() {
        let api = get_api().await;

        let req = GetRanking {
            mode: OsuGameMode::Osu,
            kind: RankingKind::Performance,
            filter: RankingFilter::All,
            country: None,
            page: None
        };

        let res = api.get_rankings(
            &req,
            50
        ).await.unwrap();

        assert_eq!(50, res.ranking.len());

        let res = api.get_rankings(
            &req,
            10
        ).await.unwrap();

        assert_eq!(10, res.ranking.len());

        let res = api.get_rankings(
            &req,
            1
        ).await.unwrap();

        assert_eq!(1, res.ranking.len());

        let res = api.get_rankings(
            &req,
            253
        ).await.unwrap();

        assert_eq!(253, res.ranking.len());

        let res = api.get_rankings(
            &req,
            111
        ).await.unwrap();

        assert_eq!(111, res.ranking.len());
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

        let mods = OsuMods::NIGHTCORE 
            | OsuMods::HIDDEN 
            | OsuMods::HARDROCK;
        assert_eq!(mods.to_string(), "HDNCHR");
    }

    #[test]
    fn test_mods_from_str() {
        // Contains one non existent mod
        // Should output DT | EZ
        let mods = OsuMods::from_str("dtdhez").unwrap();
        assert_eq!(mods, OsuMods::DOUBLETIME | OsuMods::EASY);

        let mods = OsuMods::from_str("DTHD").unwrap();
        assert_eq!(mods, OsuMods::DOUBLETIME | OsuMods::HIDDEN);

        let mods = OsuMods::from_str("DTHD").unwrap();
        assert_eq!(mods, OsuMods::DOUBLETIME | OsuMods::HIDDEN);

        let mods = OsuMods::from_str("DThDhR").unwrap();
        assert_eq!(
            mods, 
            OsuMods::DOUBLETIME 
            | OsuMods::HIDDEN 
            | OsuMods::HARDROCK
        );

        let mods = OsuMods::from_str("DThDhRdt").unwrap();
        assert_eq!(
            mods, 
            OsuMods::DOUBLETIME 
            | OsuMods::HIDDEN 
            | OsuMods::HARDROCK
        );

        let mods = OsuMods::from_str("DTMR").unwrap();
        assert_eq!(
            mods, 
            OsuMods::DOUBLETIME 
            | OsuMods::MIRROR
        );
    }
}
